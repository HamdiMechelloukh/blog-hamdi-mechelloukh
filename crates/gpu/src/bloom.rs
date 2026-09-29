//! Post-traitement bloom : la scène est rendue dans une texture HDR, dont les zones lumineuses sont
//! extraites et floutées à demi-résolution, puis ajoutées à l'image finale dans le canvas.

pub const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
/// Aller-retours de flou (horizontal + vertical) : chacun élargit le halo.
const BLUR_ITERATIONS: usize = 2;

pub struct Bloom {
    single_layout: wgpu::BindGroupLayout,
    composite_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    bright_pipeline: wgpu::RenderPipeline,
    blur_horizontal_pipeline: wgpu::RenderPipeline,
    blur_vertical_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,
    targets: Targets,
}

/// Textures et bind groups dépendant de la taille du canvas, recréés au resize.
struct Targets {
    hdr: wgpu::TextureView,
    /// Ping-pong du flou, à demi-résolution.
    half: [wgpu::TextureView; 2],
    bright_bind_group: wgpu::BindGroup,
    /// Lire half[0] (écrire half[1]), puis lire half[1] (écrire half[0]).
    blur_bind_groups: [wgpu::BindGroup; 2],
    composite_bind_group: wgpu::BindGroup,
}

impl Bloom {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat, width: u32, height: u32) -> Self {
        let texture_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let sampler_entry = wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };
        let single_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post single"),
            entries: &[texture_entry(0), sampler_entry],
        });
        let composite_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post composite"),
            entries: &[texture_entry(0), sampler_entry, texture_entry(2)],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("post"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("post"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/post.wgsl").into()),
        });
        let pipeline = |layout: &wgpu::BindGroupLayout, entry_point: &str, format: wgpu::TextureFormat| {
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[Some(layout)],
                immediate_size: 0,
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry_point),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(entry_point),
                    compilation_options: Default::default(),
                    targets: &[Some(format.into())],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let bright_pipeline = pipeline(&single_layout, "fs_bright", HDR_FORMAT);
        let blur_horizontal_pipeline = pipeline(&single_layout, "fs_blur_horizontal", HDR_FORMAT);
        let blur_vertical_pipeline = pipeline(&single_layout, "fs_blur_vertical", HDR_FORMAT);
        let composite_pipeline = pipeline(&composite_layout, "fs_composite", surface_format);

        let targets = Targets::new(device, &single_layout, &composite_layout, &sampler, width, height);
        Bloom {
            single_layout,
            composite_layout,
            sampler,
            bright_pipeline,
            blur_horizontal_pipeline,
            blur_vertical_pipeline,
            composite_pipeline,
            targets,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.targets = Targets::new(device, &self.single_layout, &self.composite_layout, &self.sampler, width, height);
    }

    /// Texture dans laquelle la scène doit être rendue.
    pub fn hdr_view(&self) -> &wgpu::TextureView {
        &self.targets.hdr
    }

    /// Enchaîne extraction, flous et composition vers `output` (la texture du canvas).
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder, output: &wgpu::TextureView) {
        let targets = &self.targets;
        fullscreen_pass(encoder, &targets.half[0], &self.bright_pipeline, &targets.bright_bind_group);
        for _ in 0..BLUR_ITERATIONS {
            fullscreen_pass(encoder, &targets.half[1], &self.blur_horizontal_pipeline, &targets.blur_bind_groups[0]);
            fullscreen_pass(encoder, &targets.half[0], &self.blur_vertical_pipeline, &targets.blur_bind_groups[1]);
        }
        fullscreen_pass(encoder, output, &self.composite_pipeline, &targets.composite_bind_group);
    }
}

impl Targets {
    fn new(
        device: &wgpu::Device,
        single_layout: &wgpu::BindGroupLayout,
        composite_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        width: u32,
        height: u32,
    ) -> Self {
        let texture = |label, width: u32, height: u32| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d { width: width.max(1), height: height.max(1), depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: HDR_FORMAT,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let hdr = texture("hdr", width, height);
        let half = [texture("bloom a", width / 2, height / 2), texture("bloom b", width / 2, height / 2)];

        let single = |view: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: single_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
                ],
            })
        };
        let composite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: composite_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&hdr) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&half[0]) },
            ],
        });
        Targets {
            bright_bind_group: single(&hdr),
            blur_bind_groups: [single(&half[0]), single(&half[1])],
            composite_bind_group,
            hdr,
            half,
        }
    }
}

fn fullscreen_pass(
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    bind_group: &wgpu::BindGroup,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
        })],
        ..Default::default()
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bind_group, &[]);
    pass.draw(0..3, 0..1);
}
