//! Rendu WebGPU du site : un canvas plein écran derrière la page, qui dessine autour des ancres DOM
//! (`data-gpu="panel|card|title|target"`). Sans WebGPU, rien n'est créé et le CSS seul s'applique.
//! `<body data-gpu-mode>` : "calm" sur les articles, "game" sur la page 404 (voir game.rs).

mod bloom;
mod game;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use bytemuck::{Pod, Zeroable};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{Element, HtmlCanvasElement, PointerEvent, Window};

const MAX_RECTS: usize = 128;
const PARTICLES_DESKTOP: u32 = 40_000;
const PARTICLES_MOBILE: u32 = 15_000;
const MOBILE_WIDTH: f64 = 768.0;
const MAX_DPR: f64 = 2.0;
/// Pointeur hors écran : aucune attraction ni lueur.
const POINTER_AWAY: [f32; 2] = [-1.0e5, -1.0e5];
/// Intensité de l'animation sur les pages de lecture (`data-gpu-mode="calm"`).
const CALM_INTENSITY: f32 = 0.12;

const COMMON: &str = include_str!("shaders/common.wgsl");

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    resolution: [f32; 2],
    pointer: [f32; 2],
    time: f32,
    dt: f32,
    scroll_delta: f32,
    intensity: f32,
    rect_count: u32,
    dpr: f32,
    _pad: [f32; 2],
    shock: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Rect {
    min: [f32; 2],
    max: [f32; 2],
    kind: u32,
    glow: f32,
    _pad: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Particle {
    pos: [f32; 2],
    vel: [f32; 2],
    slot: [f32; 2],
    captured: f32,
    _pad: f32,
}

struct Anchor {
    element: Element,
    kind: u32,
    /// Intensité du survol, lissée d'une frame à l'autre.
    glow: f32,
}

struct Renderer {
    window: Window,
    canvas: HtmlCanvasElement,
    // Sur le web, l'instance doit survivre à la surface et au device.
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    globals_buffer: wgpu::Buffer,
    rects_buffer: wgpu::Buffer,
    compute_bind_group: wgpu::BindGroup,
    render_bind_group: wgpu::BindGroup,
    compute_pipeline: wgpu::ComputePipeline,
    scene_pipeline: wgpu::RenderPipeline,
    particles_pipeline: wgpu::RenderPipeline,
    bloom: bloom::Bloom,
    particle_count: u32,
    /// Présent seulement sur la page 404.
    game: Option<game::Game>,
    anchors: Vec<Anchor>,
    pointer: Rc<Cell<[f32; 2]>>,
    /// Clic pas encore transformé en onde de choc.
    pending_click: Rc<Cell<Option<[f32; 2]>>>,
    shock: [f32; 4],
    intensity: f32,
    reduced_motion: bool,
    last_time: f64,
    last_scroll: f64,
}

#[wasm_bindgen(start)]
fn start() {
    wasm_bindgen_futures::spawn_local(async {
        if let Err(error) = run().await {
            // Pas de WebGPU ou échec d'init : le site reste en CSS seul.
            web_sys::console::warn_1(&error);
        }
    });
}

async fn run() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("pas de window")?;
    let has_webgpu = js_sys::Reflect::has(&window.navigator(), &"gpu".into())?;
    if !has_webgpu {
        return Err("WebGPU indisponible".into());
    }
    let renderer = Rc::new(RefCell::new(Renderer::new(window.clone()).await?));

    // La page passe en mode GPU seulement quand le rendu est prêt (le CSS retire alors ses fonds).
    let document = window.document().ok_or("pas de document")?;
    document.document_element().ok_or("pas de <html>")?.class_list().add_1("gpu")?;

    // Boucle requestAnimationFrame : elle se suspend d'elle-même quand l'onglet est caché.
    let frame: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let next_frame = frame.clone();
    let loop_window = window.clone();
    *frame.borrow_mut() = Some(Closure::new(move |now: f64| {
        renderer.borrow_mut().frame(now);
        if let Some(callback) = next_frame.borrow().as_ref() {
            let _ = loop_window.request_animation_frame(callback.as_ref().unchecked_ref());
        }
    }));
    window.request_animation_frame(frame.borrow().as_ref().unwrap().as_ref().unchecked_ref())?;
    Ok(())
}

impl Renderer {
    async fn new(window: Window) -> Result<Self, JsValue> {
        let document = window.document().ok_or("pas de document")?;
        let body = document.body().ok_or("pas de <body>")?;

        let canvas: HtmlCanvasElement = document.create_element("canvas")?.dyn_into()?;
        canvas.set_class_name("gpu-canvas");
        canvas.set_attribute("aria-hidden", "true")?;
        body.prepend_with_node_1(&canvas)?;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|error| error.to_string())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .map_err(|error| error.to_string())?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .map_err(|error| error.to_string())?;

        // Taille réelle fixée à la première frame (resize).
        let config = surface.get_default_config(&adapter, 1, 1).ok_or("surface non configurable")?;
        let format = config.format;

        let mode = body.get_attribute("data-gpu-mode");
        let reduced_motion = window
            .match_media("(prefers-reduced-motion: reduce)")?
            .is_some_and(|query| query.matches());
        // Sans mouvement, le jeu est injouable : la 404 garde alors son rendu calme.
        let is_game = mode.as_deref() == Some("game") && !reduced_motion;

        let viewport_width = window.inner_width()?.as_f64().unwrap_or(1024.0);
        let viewport_height = window.inner_height()?.as_f64().unwrap_or(768.0);
        let (particle_count, slots) = if is_game {
            (game::PARTICLES, game::glyph_slots(&document).await?)
        } else if viewport_width < MOBILE_WIDTH {
            (PARTICLES_MOBILE, vec![[0.0; 2]])
        } else {
            (PARTICLES_DESKTOP, vec![[0.0; 2]])
        };
        let particles = seed_particles(particle_count, viewport_width as f32, viewport_height as f32, &slots);

        use wgpu::util::DeviceExt;
        let globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let rects_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rects"),
            size: (size_of::<Rect>() * MAX_RECTS) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let particles_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("particles"),
            contents: bytemuck::cast_slice(&particles),
            usage: wgpu::BufferUsages::STORAGE,
        });
        // Compteur de captures de la page 404 ; lié partout pour garder un seul layout de compute.
        let captured_counter = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("captured"),
            contents: &[0; 4],
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });

        let compute_layout = bind_group_layout(&device, "compute", true);
        let render_layout = bind_group_layout(&device, "render", false);
        let mut entries = vec![
            wgpu::BindGroupEntry { binding: 0, resource: globals_buffer.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: rects_buffer.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: particles_buffer.as_entire_binding() },
        ];
        let render_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &render_layout, entries: &entries });
        entries.push(wgpu::BindGroupEntry { binding: 3, resource: captured_counter.as_entire_binding() });
        let compute_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &compute_layout, entries: &entries });

        let compute_module = shader(&device, "compute", include_str!("shaders/compute.wgsl"));
        let scene_module = shader(&device, "scene", include_str!("shaders/scene.wgsl"));
        let particles_module = shader(&device, "particles", include_str!("shaders/particles.wgsl"));

        let compute_pipeline_layout = pipeline_layout(&device, &compute_layout);
        let compute_pipeline = create_compute_pipeline(&device, &compute_pipeline_layout, &compute_module);
        let game = if is_game {
            let module = shader(&device, "game", include_str!("shaders/game.wgsl"));
            let pipeline = create_compute_pipeline(&device, &compute_pipeline_layout, &module);
            Some(game::Game::new(&device, pipeline, captured_counter, &document)?)
        } else {
            None
        };
        let render_pipeline_layout = pipeline_layout(&device, &render_layout);
        // Scène et particules sont rendues en HDR ; le bloom compose ensuite dans le format du canvas.
        let scene_pipeline =
            render_pipeline(&device, &render_pipeline_layout, &scene_module, bloom::HDR_FORMAT, None);
        let additive = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        };
        let particles_pipeline =
            render_pipeline(&device, &render_pipeline_layout, &particles_module, bloom::HDR_FORMAT, Some(additive));
        let bloom = bloom::Bloom::new(&device, format, config.width, config.height);

        let pointer = Rc::new(Cell::new(POINTER_AWAY));
        let pending_click = Rc::new(Cell::new(None));
        listen_pointer(&window, pointer.clone(), pending_click.clone())?;

        let anchor_nodes = document.query_selector_all("[data-gpu]")?;
        let anchors = (0..anchor_nodes.length())
            .filter_map(|index| anchor_nodes.item(index)?.dyn_into::<Element>().ok())
            .filter_map(|element| {
                let kind = match element.get_attribute("data-gpu")?.as_str() {
                    "panel" => 0,
                    "card" => 1,
                    "title" => 2,
                    "target" => 3,
                    _ => return None,
                };
                Some(Anchor { element, kind, glow: 0.0 })
            })
            .take(MAX_RECTS)
            .collect();

        Ok(Renderer {
            last_scroll: window.scroll_y()?,
            window,
            canvas,
            _instance: instance,
            surface,
            device,
            queue,
            config,
            globals_buffer,
            rects_buffer,
            compute_bind_group,
            render_bind_group,
            compute_pipeline,
            scene_pipeline,
            particles_pipeline,
            bloom,
            particle_count,
            game,
            anchors,
            pointer,
            pending_click,
            shock: [0.0; 4],
            intensity: if mode.as_deref() == Some("calm") { CALM_INTENSITY } else { 1.0 },
            reduced_motion,
            last_time: 0.0,
        })
    }

    fn frame(&mut self, now: f64) {
        let dpr = self.window.device_pixel_ratio().min(MAX_DPR);
        let width = self.window.inner_width().ok().and_then(|value| value.as_f64()).unwrap_or(1.0);
        let height = self.window.inner_height().ok().and_then(|value| value.as_f64()).unwrap_or(1.0);
        self.resize((width * dpr) as u32, (height * dpr) as u32);

        // dt borné : après un onglet caché, pas de saut de simulation.
        let dt = if self.last_time == 0.0 { 0.0 } else { ((now - self.last_time) / 1000.0).min(0.05) } as f32;
        self.last_time = now;
        let scroll = self.window.scroll_y().unwrap_or(0.0);
        let scroll_delta = (scroll - self.last_scroll) as f32;
        self.last_scroll = scroll;

        let pointer = self.pointer.get();
        let rects = self.collect_rects(pointer, dt);
        let frozen = self.reduced_motion;
        let time = (now / 1000.0) as f32;
        if let Some([x, y]) = self.pending_click.take() {
            if !frozen {
                self.shock = [x, y, time, 1.0];
            }
        }
        let globals = Globals {
            resolution: [width as f32, height as f32],
            pointer,
            time: if frozen { 0.0 } else { time },
            dt: if frozen { 0.0 } else { dt },
            scroll_delta,
            intensity: self.intensity,
            rect_count: rects.len() as u32,
            dpr: dpr as f32,
            _pad: [0.0; 2],
            shock: self.shock,
        };
        self.queue.write_buffer(&self.globals_buffer, 0, bytemuck::bytes_of(&globals));
        if !rects.is_empty() {
            self.queue.write_buffer(&self.rects_buffer, 0, bytemuck::cast_slice(&rects));
        }

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            // Frame sautée : la suivante réessaiera.
            _ => return,
        };
        let view = frame.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(self.game.as_ref().map_or(&self.compute_pipeline, |game| &game.pipeline));
            pass.set_bind_group(0, &self.compute_bind_group, &[]);
            pass.dispatch_workgroups(self.particle_count.div_ceil(64), 1, 1);
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.bloom.hdr_view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            pass.set_bind_group(0, &self.render_bind_group, &[]);
            pass.set_pipeline(&self.scene_pipeline);
            pass.draw(0..3, 0..1);
            pass.set_pipeline(&self.particles_pipeline);
            pass.draw(0..6, 0..self.particle_count);
        }
        self.bloom.encode(&mut encoder, &view);
        let readback = self.game.as_mut().is_some_and(|game| game.encode_readback(&mut encoder));
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
        if let Some(game) = &mut self.game {
            if readback {
                game.start_readback();
            }
            game.update_dom();
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        let (width, height) = (width.max(1), height.max(1));
        if width == self.config.width && height == self.config.height {
            return;
        }
        self.canvas.set_width(width);
        self.canvas.set_height(height);
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.bloom.resize(&self.device, width, height);
    }

    /// Positions des ancres visibles dans le viewport, avec le survol lissé.
    fn collect_rects(&mut self, pointer: [f32; 2], dt: f32) -> Vec<Rect> {
        let viewport_height = self.config.height as f32 / self.window.device_pixel_ratio().min(MAX_DPR) as f32;
        let easing = (dt * 8.0).min(1.0);
        let mut rects = Vec::with_capacity(self.anchors.len());
        for anchor in &mut self.anchors {
            let bounds = anchor.element.get_bounding_client_rect();
            let (min, max) = (
                [bounds.left() as f32, bounds.top() as f32],
                [bounds.right() as f32, bounds.bottom() as f32],
            );
            let hovered = anchor.kind == 1
                && (min[0]..max[0]).contains(&pointer[0])
                && (min[1]..max[1]).contains(&pointer[1]);
            anchor.glow += (f32::from(u8::from(hovered)) - anchor.glow) * easing;
            // Hors écran (avec marge pour les halos) : inutile de l'envoyer au GPU.
            if max[1] < -100.0 || min[1] > viewport_height + 100.0 {
                continue;
            }
            rects.push(Rect { min, max, kind: anchor.kind, glow: anchor.glow, _pad: [0.0; 2] });
        }
        rects
    }
}

fn listen_pointer(
    window: &Window,
    pointer: Rc<Cell<[f32; 2]>>,
    pending_click: Rc<Cell<Option<[f32; 2]>>>,
) -> Result<(), JsValue> {
    let on_down = Closure::<dyn FnMut(PointerEvent)>::new(move |event: PointerEvent| {
        pending_click.set(Some([event.client_x() as f32, event.client_y() as f32]));
    });
    window.add_event_listener_with_callback("pointerdown", on_down.as_ref().unchecked_ref())?;
    on_down.forget();
    let move_pointer = pointer.clone();
    let on_move = Closure::<dyn FnMut(PointerEvent)>::new(move |event: PointerEvent| {
        move_pointer.set([event.client_x() as f32, event.client_y() as f32]);
    });
    let on_leave = Closure::<dyn FnMut()>::new(move || pointer.set(POINTER_AWAY));
    window.add_event_listener_with_callback("pointermove", on_move.as_ref().unchecked_ref())?;
    window
        .document()
        .ok_or("pas de document")?
        .add_event_listener_with_callback("pointerleave", on_leave.as_ref().unchecked_ref())?;
    // Écouteurs actifs pendant toute la vie de la page.
    on_move.forget();
    on_leave.forget();
    Ok(())
}

/// Répartition pseudo-aléatoire (xorshift) : pas besoin d'une dépendance `rand` pour ça.
fn seed_particles(count: u32, width: f32, height: f32, slots: &[[f32; 2]]) -> Vec<Particle> {
    let mut state: u32 = 0x9e37_79b9;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        state as f32 / u32::MAX as f32
    };
    (0..count as usize)
        .map(|index| Particle {
            pos: [next() * width, next() * height],
            vel: [0.0, 0.0],
            // Pas premier : répartit les particules sur tout le masque plutôt que ligne par ligne.
            slot: slots[index * 7919 % slots.len()],
            captured: 0.0,
            _pad: 0.0,
        })
        .collect()
}

fn shader(device: &wgpu::Device, label: &str, source: &str) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(format!("{COMMON}\n{source}").into()),
    })
}

/// 0 : globals, 1 : ancres, 2 : particules (écriture pour le compute, lecture pour le rendu),
/// 3 : compteur de captures (compute seulement).
fn bind_group_layout(device: &wgpu::Device, label: &str, compute: bool) -> wgpu::BindGroupLayout {
    let storage = |binding, read_only| wgpu::BindGroupLayoutEntry {
        binding,
        // Un storage inscriptible visible du vertex shader exigerait la feature VERTEX_WRITABLE_STORAGE.
        visibility: if read_only { wgpu::ShaderStages::all() } else { wgpu::ShaderStages::COMPUTE },
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };
    let mut entries = vec![
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::all(),
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        storage(1, true),
        storage(2, !compute),
    ];
    if compute {
        entries.push(storage(3, false));
    }
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some(label), entries: &entries })
}

fn create_compute_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    module: &wgpu::ShaderModule,
) -> wgpu::ComputePipeline {
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(layout),
        module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    })
}

fn pipeline_layout(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    })
}

fn render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    module: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    blend: Option<wgpu::BlendState>,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask: wgpu::ColorWrites::ALL })],
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}
