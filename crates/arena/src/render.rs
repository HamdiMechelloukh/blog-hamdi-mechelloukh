//! Rendu WebGPU de l'arène : formes lumineuses instanciées et particules d'impact.

use bytemuck::{Pod, Zeroable};
use web_sys::HtmlCanvasElement;

use crate::loadout::Weapon;
use crate::rng::Rng;
use crate::sim::{ARENA_HEIGHT, ARENA_WIDTH, Event, Fight, PLAYER, ROBOT_RADIUS};
use crate::vec2::Vec2;

const MAX_DPR: f64 = 2.0;
/// Fond de la page (--bg-color) : l'arène s'y fond.
const BACKGROUND: wgpu::Color = wgpu::Color { r: 0.0196, g: 0.0235, b: 0.047, a: 1.0 };
/// L'orange de la charte pour le joueur, un bleu froid pour l'IA.
const PLAYER_COLOR: [f32; 3] = [0.976, 0.451, 0.086];
const ENEMY_COLOR: [f32; 3] = [0.3, 0.75, 1.0];
const SHIELD_COLOR: [f32; 3] = [0.65, 0.9, 1.0];
const SPARK_COLOR: [f32; 3] = [1.0, 0.72, 0.35];

// Formes de arena.wgsl.
const SHAPE_ARENA: f32 = 0.0;
const SHAPE_DISC: f32 = 1.0;
const SHAPE_RING: f32 = 2.0;
const SHAPE_BOX: f32 = 3.0;

const SHIELD_RADIUS: f32 = 27.0;
const HEALTH_BAR_HALF_WIDTH: f32 = 22.0;
const HEALTH_BAR_OFFSET: f32 = 34.0;
const PARTICLE_DRAG: f32 = 3.0;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    arena: [f32; 2],
    pixel: f32,
    time: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Instance {
    center: [f32; 2],
    half_size: [f32; 2],
    color: [f32; 4],
    /// angle, forme, paramètres de la forme.
    params: [f32; 4],
}

impl Instance {
    fn new(center: Vec2, half_size: [f32; 2], color: [f32; 3], alpha: f32, params: [f32; 4]) -> Self {
        Self { center: [center.x, center.y], half_size, color: [color[0], color[1], color[2], alpha], params }
    }

    fn disc(center: Vec2, radius: f32, color: [f32; 3], alpha: f32) -> Self {
        Self::new(center, [radius, radius], color, alpha, [0.0, SHAPE_DISC, 0.0, 0.0])
    }

    fn ring(center: Vec2, radius: f32, thickness: f32, color: [f32; 3], alpha: f32) -> Self {
        Self::new(center, [radius, radius], color, alpha, [0.0, SHAPE_RING, thickness, 0.0])
    }

    fn rect(center: Vec2, half_size: [f32; 2], angle: f32, corner: f32, fill: f32, color: [f32; 3]) -> Self {
        Self::new(center, half_size, color, 1.0, [angle, SHAPE_BOX, corner, fill])
    }
}

struct Particle {
    position: Vec2,
    velocity: Vec2,
    life: f32,
    duration: f32,
    size: f32,
    color: [f32; 3],
    /// Onde de choc qui s'élargit plutôt qu'étincelle qui rétrécit.
    ring: bool,
}

pub struct Renderer {
    canvas: HtmlCanvasElement,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    globals: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    instances: wgpu::Buffer,
    particles: Vec<Particle>,
    rng: Rng,
    time: f32,
}

impl Renderer {
    pub async fn new(canvas: HtmlCanvasElement, seed: u64) -> Result<Self, String> {
        // WebGPU seul : le repli WebGL2 multipliait le module par 16 (1 Mo gzip contre 67 Ko).
        let window = web_sys::window().ok_or("pas de window")?;
        if !js_sys::Reflect::has(&window.navigator(), &"gpu".into()).unwrap_or(false) {
            return Err("WebGPU indisponible".into());
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface =
            instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone())).map_err(|error| error.to_string())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions { compatible_surface: Some(&surface), ..Default::default() })
            .await
            .map_err(|error| error.to_string())?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .map_err(|error| error.to_string())?;

        let mut config = surface.get_default_config(&adapter, 1, 1).ok_or("surface non configurable")?;
        // Couleurs écrites telles quelles (comme le CSS) : format non sRGB quand il existe.
        if let Some(format) = surface.get_capabilities(&adapter).formats.into_iter().find(|format| !format.is_srgb()) {
            config.format = format;
        }

        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("arena"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/arena.wgsl").into()),
        });
        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("arena"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("arena"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals.as_entire_binding() }],
        });
        let additive =
            wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, ..Default::default() };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("arena"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            })),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<Instance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState { color: additive, alpha: additive }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleStrip, ..Default::default() },
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let instances = instance_buffer(&device, 256);

        Ok(Self {
            canvas,
            surface,
            device,
            queue,
            config,
            pipeline,
            globals,
            bind_group,
            instances,
            particles: Vec::new(),
            rng: Rng::new(seed),
            time: 0.0,
        })
    }

    /// Étincelles, éclairs de bouche et explosions déclenchés par le dernier pas de simulation.
    pub fn spawn_effects(&mut self, events: &[Event]) {
        for event in events {
            match *event {
                Event::Shot { owner, position } => self.burst(position, 3, 60.0, 0.15, 2.0, hot(robot_color(owner))),
                Event::Hit { position, shielded } => {
                    self.burst(position, 12, 220.0, 0.45, 2.2, if shielded { SHIELD_COLOR } else { SPARK_COLOR })
                }
                Event::Slash { owner, position } => self.shockwave(position, 0.25, robot_color(owner)),
                Event::Destroyed { position } => {
                    self.burst(position, 70, 320.0, 1.2, 3.0, SPARK_COLOR);
                    self.shockwave(position, 0.6, SPARK_COLOR);
                }
            }
        }
    }

    fn burst(&mut self, position: Vec2, count: usize, speed: f32, duration: f32, size: f32, color: [f32; 3]) {
        for _ in 0..count {
            let velocity = Vec2::from_angle(self.rng.unit() * std::f32::consts::TAU) * (speed * (0.3 + 0.7 * self.rng.unit()));
            let duration = duration * (0.5 + 0.5 * self.rng.unit());
            self.particles.push(Particle { position, velocity, life: duration, duration, size, color, ring: false });
        }
    }

    fn shockwave(&mut self, position: Vec2, duration: f32, color: [f32; 3]) {
        self.particles.push(Particle { position, velocity: Vec2::ZERO, life: duration, duration, size: 40.0, color, ring: true });
    }

    pub fn draw(&mut self, fight: &Fight, dt: f32) {
        self.time += dt;
        self.update_particles(fight, dt);
        let window = web_sys::window().expect("window");
        let dpr = window.device_pixel_ratio().min(MAX_DPR);
        let width = ((self.canvas.client_width() as f64 * dpr) as u32).max(1);
        self.resize(width, (width as f32 * ARENA_HEIGHT / ARENA_WIDTH) as u32);

        let instances = self.instances_for(fight);
        let needed = (instances.len() * size_of::<Instance>()) as u64;
        if needed > self.instances.size() {
            self.instances = instance_buffer(&self.device, instances.len().next_power_of_two());
        }
        self.queue.write_buffer(&self.instances, 0, bytemuck::cast_slice(&instances));
        let globals =
            Globals { arena: [ARENA_WIDTH, ARENA_HEIGHT], pixel: ARENA_WIDTH / self.config.width as f32, time: self.time };
        self.queue.write_buffer(&self.globals, 0, bytemuck::bytes_of(&globals));

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            _ => return,
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("arena"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(BACKGROUND), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.instances.slice(..needed));
            pass.draw(0..4, 0..instances.len() as u32);
        }
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
    }

    fn update_particles(&mut self, fight: &Fight, dt: f32) {
        // Traînée des missiles.
        for projectile in &fight.projectiles {
            if projectile.weapon == Weapon::Missiles {
                let drift = Vec2::from_angle(self.rng.unit() * std::f32::consts::TAU) * 15.0;
                let color = hot(robot_color(projectile.owner));
                self.particles.push(Particle {
                    position: projectile.position,
                    velocity: drift,
                    life: 0.35,
                    duration: 0.35,
                    size: 1.8,
                    color,
                    ring: false,
                });
            }
        }
        for particle in &mut self.particles {
            particle.position = particle.position + particle.velocity * dt;
            particle.velocity = particle.velocity * (1.0 - PARTICLE_DRAG * dt).max(0.0);
            particle.life -= dt;
        }
        self.particles.retain(|particle| particle.life > 0.0);
    }

    fn instances_for(&self, fight: &Fight) -> Vec<Instance> {
        let mut instances = vec![Instance::new(
            Vec2::new(ARENA_WIDTH / 2.0, ARENA_HEIGHT / 2.0),
            [ARENA_WIDTH / 2.0 - 3.0, ARENA_HEIGHT / 2.0 - 3.0],
            PLAYER_COLOR,
            0.35,
            [0.0, SHAPE_ARENA, 0.0, 0.0],
        )];
        for (index, robot) in fight.robots.iter().enumerate() {
            if robot.health <= 0.0 {
                continue;
            }
            let color = robot_color(index);
            let heading = Vec2::from_angle(robot.heading);
            let body = ROBOT_RADIUS - 4.0;
            instances.push(Instance::rect(robot.position, [body, body], robot.heading, 5.0, 0.12, color));
            // Canon(s) : la silhouette dit l'arme.
            let (length, width, offsets): (f32, f32, &[f32]) = match robot.loadout.weapon {
                Weapon::Cannon => (10.0, 3.5, &[0.0]),
                Weapon::Laser => (14.0, 2.0, &[0.0]),
                Weapon::Blade => (16.0, 1.5, &[0.0]),
                Weapon::Missiles => (7.0, 3.0, &[-6.0, 6.0]),
            };
            for offset in offsets {
                let center = robot.position + heading * (body + length) + heading.perpendicular() * *offset;
                instances.push(Instance::rect(center, [length, width], robot.heading, 1.0, 0.6, color));
            }
            if robot.shield_time > 0.0 {
                let alpha = (robot.shield_time * 4.0).min(1.0);
                instances.push(Instance::ring(robot.position, SHIELD_RADIUS, 1.2, SHIELD_COLOR, alpha));
            }
            let bar = robot.position - Vec2::new(0.0, HEALTH_BAR_OFFSET);
            instances.push(Instance::rect(bar, [HEALTH_BAR_HALF_WIDTH, 2.5], 0.0, 1.0, 0.25, dim(color)));
            let filled = HEALTH_BAR_HALF_WIDTH * robot.health_ratio();
            let left = bar - Vec2::new(HEALTH_BAR_HALF_WIDTH - filled, 0.0);
            instances.push(Instance::rect(left, [filled, 2.5], 0.0, 1.0, 1.0, color));
        }
        for projectile in &fight.projectiles {
            let color = hot(robot_color(projectile.owner));
            instances.push(match projectile.weapon {
                Weapon::Laser => Instance::rect(projectile.position, [9.0, 1.0], projectile.velocity.angle(), 1.0, 1.0, color),
                Weapon::Missiles => Instance::disc(projectile.position, 3.5, color, 1.0),
                Weapon::Cannon | Weapon::Blade => Instance::disc(projectile.position, 3.0, color, 1.0),
            });
        }
        for particle in &self.particles {
            let progress = particle.life / particle.duration;
            instances.push(if particle.ring {
                Instance::ring(particle.position, particle.size * (1.0 - progress), 1.0, particle.color, progress)
            } else {
                Instance::disc(particle.position, particle.size * progress, particle.color, progress)
            });
        }
        instances
    }

    fn resize(&mut self, width: u32, height: u32) {
        let height = height.max(1);
        if width == self.config.width && height == self.config.height {
            return;
        }
        self.canvas.set_width(width);
        self.canvas.set_height(height);
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }
}

fn instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("instances"),
        size: (capacity * size_of::<Instance>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn robot_color(index: usize) -> [f32; 3] {
    if index == PLAYER { PLAYER_COLOR } else { ENEMY_COLOR }
}

/// Plus clair, pour les tirs : on les distingue du robot qui les lance.
fn hot(color: [f32; 3]) -> [f32; 3] {
    color.map(|channel| channel + (1.0 - channel) * 0.5)
}

fn dim(color: [f32; 3]) -> [f32; 3] {
    color.map(|channel| channel * 0.4)
}
