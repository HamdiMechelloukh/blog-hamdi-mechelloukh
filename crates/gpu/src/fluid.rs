//! Simulation de fluide (shaders/fluid.wgsl) sur une grille fixe, dont le champ de vitesse transporte les particules.
//!
//! Tampons ping-pong : WebGPU interdit de lire et d'écrire le même buffer dans un dispatch. Deux bind groups
//! croisent les rôles (A lit a et écrit b, B lit b et écrit a). Séquence d'une frame :
//!   advect (A : vitesse a -> b), divergence (B : lit b), Jacobi alterné A, B, A… (pression a <-> b),
//!   projection (B : vitesse b -> a, pression b). Avec un nombre impair d'itérations, la dernière écrit b,
//!   que la projection lit : la vitesse finale est dans a, lue par les particules.

/// Nombre de cellules sur le grand côté ; l'autre suit les proportions de l'écran au démarrage.
const GRID_LONG_SIDE: u32 = 160;
const GRID_MIN_SHORT_SIDE: u32 = 32;
/// Impair : la pression finale doit être dans b (voir plus haut).
const JACOBI_ITERATIONS: usize = 21;
const WORKGROUP_SIZE: u32 = 8;

pub struct Fluid {
    pub grid: [u32; 2],
    /// Vitesse finale de chaque frame (tampon a), lue par le compute des particules.
    pub velocity: wgpu::Buffer,
    advect: wgpu::ComputePipeline,
    divergence: wgpu::ComputePipeline,
    jacobi: wgpu::ComputePipeline,
    project: wgpu::ComputePipeline,
    bind_groups: [wgpu::BindGroup; 2],
}

impl Fluid {
    pub fn new(
        device: &wgpu::Device,
        module: &wgpu::ShaderModule,
        globals_buffer: &wgpu::Buffer,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Self {
        let short_side = |long: f32, short: f32| ((GRID_LONG_SIDE as f32 * short / long).round() as u32).max(GRID_MIN_SHORT_SIDE);
        let grid = if viewport_width >= viewport_height {
            [GRID_LONG_SIDE, short_side(viewport_width, viewport_height)]
        } else {
            [short_side(viewport_height, viewport_width), GRID_LONG_SIDE]
        };
        let cells = u64::from(grid[0] * grid[1]);

        // WebGPU initialise les buffers à zéro : fluide au repos, pression nulle.
        let buffer = |label, bytes_per_cell: u64| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: cells * bytes_per_cell,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        };
        let velocity = [buffer("fluid velocity a", 8), buffer("fluid velocity b", 8)];
        let pressure = [buffer("fluid pressure a", 4), buffer("fluid pressure b", 4)];
        let divergence_buffer = buffer("fluid divergence", 4);

        let storage = |binding, read_only| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fluid"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                storage(2, true),
                storage(3, false),
                storage(4, true),
                storage(5, false),
                storage(6, false),
            ],
        });
        // Groupe `read` : lit les tampons d'indice `read`, écrit les autres.
        let bind_group = |read: usize| {
            let write = 1 - read;
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: globals_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: velocity[read].as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 3, resource: velocity[write].as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 4, resource: pressure[read].as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 5, resource: pressure[write].as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 6, resource: divergence_buffer.as_entire_binding() },
                ],
            })
        };
        let bind_groups = [bind_group(0), bind_group(1)];

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("fluid"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |entry_point| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry_point),
                layout: Some(&pipeline_layout),
                module,
                entry_point: Some(entry_point),
                compilation_options: Default::default(),
                cache: None,
            })
        };

        // Les autres tampons restent vivants tant que les bind groups les référencent.
        let [velocity_a, _] = velocity;
        Fluid {
            grid,
            velocity: velocity_a,
            advect: pipeline("advect"),
            divergence: pipeline("compute_divergence"),
            jacobi: pipeline("jacobi"),
            project: pipeline("project"),
            bind_groups,
        }
    }

    /// Une étape de simulation, à encoder avant le compute des particules (qui lit `velocity`).
    pub fn encode(&self, pass: &mut wgpu::ComputePass) {
        let [a, b] = &self.bind_groups;
        let mut dispatch = |pipeline: &wgpu::ComputePipeline, bind_group: &wgpu::BindGroup| {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.dispatch_workgroups(self.grid[0].div_ceil(WORKGROUP_SIZE), self.grid[1].div_ceil(WORKGROUP_SIZE), 1);
        };
        dispatch(&self.advect, a);
        dispatch(&self.divergence, b);
        for iteration in 0..JACOBI_ITERATIONS {
            dispatch(&self.jacobi, if iteration % 2 == 0 { a } else { b });
        }
        dispatch(&self.project, b);
    }
}
