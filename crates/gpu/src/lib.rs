//! Rendu WebGPU du site : un canvas plein écran derrière la page, qui dessine autour des ancres DOM
//! (`data-gpu="panel|card|title|target"`). Sans WebGPU, rien n'est créé et le CSS seul s'applique.
//! `<body data-gpu-mode>` : "calm" sur les articles, "game" sur la page 404 (voir game.rs).

mod bloom;
mod fluid;
mod game;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

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
/// Transition d'entrée : point du clic sur un lien de la page précédente, en sessionStorage.
const TRANSITION_KEY: &str = "gpu-transition-origin";
/// Au-delà, le clic mémorisé n'est plus celui qui a amené ici (rechargement, retour arrière…).
const TRANSITION_MAX_AGE_MS: f64 = 3000.0;
const BURST_MIN_SPEED: f32 = 300.0;
/// Aligné sur MAX_SPEED de compute.wgsl.
const BURST_MAX_SPEED: f32 = 1500.0;
/// Qualité adaptative : au-delà de ce temps de frame moyen, on dessine moins de particules.
const SLOW_FRAME_MS: f32 = 28.0;
const FRAME_BUDGET_MS: f32 = 1000.0 / 60.0;
/// Les premières frames sont lentes par nature (compilation des shaders) : elles ne comptent pas.
const WARMUP_FRAMES: u32 = 60;

const COMMON: &str = include_str!("shaders/common.wgsl");
/// Les trois étapes de WebGPU. Surtout pas `ShaderStages::all()` : il inclut des étapes propres à wgpu natif
/// (mesh, task…) que Chrome rejette (« Value 511 is invalid for WGPUShaderStage »), là où Firefox laisse passer.
const VISIBLE_EVERYWHERE: wgpu::ShaderStages =
    wgpu::ShaderStages::VERTEX.union(wgpu::ShaderStages::FRAGMENT).union(wgpu::ShaderStages::COMPUTE);

// Types d'ancres, mêmes valeurs que les constantes KIND_* de common.wgsl.
const KIND_PANEL: u32 = 0;
const KIND_CARD: u32 = 1;
const KIND_TITLE: u32 = 2;
const KIND_TARGET: u32 = 3;
const KIND_READING: u32 = 4;
const KIND_VIZ: u32 = 5;
const KIND_SCRIM: u32 = 6;
/// Valeurs de `data-viz`, dans l'ordre des constantes VIZ_* de viz.wgsl.
const VIZ_VARIANTS: [&str; 4] = ["flink", "condorcet", "agents", "lakehouse"];

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
    scroll_velocity: f32,
    reading_progress: f32,
    shock: [f32; 4],
    pointer_velocity: [f32; 2],
    fluid_grid: [u32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Rect {
    min: [f32; 2],
    max: [f32; 2],
    kind: u32,
    glow: f32,
    variant: u32,
    _pad: f32,
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
    /// Visualisation d'article : indice de `data-viz` dans VIZ_VARIANTS.
    variant: u32,
}

struct Renderer {
    window: Window,
    canvas: HtmlCanvasElement,
    // Sur le web, l'instance doit survivre à la surface et au device.
    _instance: wgpu::Instance,
    /// Levé par le callback de perte du device (qui doit être Send, d'où l'atomique).
    device_lost: Arc<AtomicBool>,
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
    /// Particules dessinées (qualité adaptative) : toutes au départ, jusqu'à un quart si les frames sont lentes.
    drawn_particles: u32,
    /// Temps de frame moyen (moyenne mobile exponentielle), en ms.
    frame_ms: f32,
    frames_rendered: u32,
    /// Présent seulement sur la page 404.
    game: Option<game::Game>,
    anchors: Vec<Anchor>,
    pointer: Rc<Cell<[f32; 2]>>,
    /// Position du curseur à la frame précédente, pour sa vitesse.
    last_pointer: [f32; 2],
    /// Vitesse du curseur lissée (px/s), qui entraîne le fluide.
    pointer_velocity: [f32; 2],
    /// Présent partout sauf sur la 404, où le jeu a son propre compute.
    fluid: fluid::Fluid,
    /// Clic pas encore transformé en onde de choc.
    pending_click: Rc<Cell<Option<[f32; 2]>>>,
    shock: [f32; 4],
    intensity: f32,
    reduced_motion: bool,
    last_time: f64,
    last_scroll: f64,
    scroll_velocity: f32,
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
    // `paused` l'arrête pendant une navigation (pagehide) ; elle reprend si la page revient du cache.
    let paused = Rc::new(Cell::new(false));
    let frame: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let next_frame = frame.clone();
    let loop_window = window.clone();
    let loop_paused = paused.clone();
    *frame.borrow_mut() = Some(Closure::new(move |now: f64| {
        if loop_paused.get() {
            return;
        }
        let mut renderer = renderer.borrow_mut();
        if renderer.device_lost.load(Ordering::Relaxed) {
            renderer.shut_down();
            return;
        }
        renderer.frame(now);
        if let Some(callback) = next_frame.borrow().as_ref() {
            let _ = loop_window.request_animation_frame(callback.as_ref().unchecked_ref());
        }
    }));
    window.request_animation_frame(frame.borrow().as_ref().unwrap().as_ref().unchecked_ref())?;

    let hide_paused = paused.clone();
    let on_page_hide = Closure::<dyn FnMut()>::new(move || hide_paused.set(true));
    let show_window = window.clone();
    let on_page_show = Closure::<dyn FnMut()>::new(move || {
        if paused.replace(false) {
            if let Some(callback) = frame.borrow().as_ref() {
                let _ = show_window.request_animation_frame(callback.as_ref().unchecked_ref());
            }
        }
    });
    window.add_event_listener_with_callback("pagehide", on_page_hide.as_ref().unchecked_ref())?;
    window.add_event_listener_with_callback("pageshow", on_page_show.as_ref().unchecked_ref())?;
    on_page_hide.forget();
    on_page_show.forget();
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
        // GPU réinitialisé (pilote surchargé, veille…) : on arrête d'envoyer des commandes vers un device mort.
        let device_lost = Arc::new(AtomicBool::new(false));
        let lost_flag = device_lost.clone();
        device.set_device_lost_callback(move |_, message| {
            web_sys::console::warn_1(&format!("device WebGPU perdu : {message}").into());
            lost_flag.store(true, Ordering::Relaxed);
        });

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
        // Jaillissement depuis le clic de la page précédente, sauf en lecture, sur la 404 ou sans mouvement.
        let origin = take_transition_origin(&window).filter(|_| mode.is_none() && !reduced_motion);
        let particles = seed_particles(particle_count, viewport_width as f32, viewport_height as f32, &slots, origin);

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
        let fluid_module = shader(&device, "fluid", include_str!("shaders/fluid.wgsl"));
        let fluid = fluid::Fluid::new(&device, &fluid_module, &globals_buffer, viewport_width as f32, viewport_height as f32);
        entries.push(wgpu::BindGroupEntry { binding: 3, resource: captured_counter.as_entire_binding() });
        entries.push(wgpu::BindGroupEntry { binding: 4, resource: fluid.velocity.as_entire_binding() });
        let compute_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &compute_layout, entries: &entries });

        let compute_module = shader(&device, "compute", include_str!("shaders/compute.wgsl"));
        let scene_module =
            shader(&device, "scene", concat!(include_str!("shaders/viz.wgsl"), include_str!("shaders/scene.wgsl")));
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
        // Fusion max et non additive : une zone dense a la luminosité d'une seule particule. En additif, les
        // superpositions saturaient au blanc et fatiguaient les yeux. WebGPU impose des facteurs One avec Max.
        let brightest = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Max,
            },
            alpha: wgpu::BlendComponent::OVER,
        };
        let particles_pipeline =
            render_pipeline(&device, &render_pipeline_layout, &particles_module, bloom::HDR_FORMAT, Some(brightest));
        let bloom = bloom::Bloom::new(&device, format, config.width, config.height);

        let pointer = Rc::new(Cell::new(POINTER_AWAY));
        // L'onde de choc accompagne le jaillissement dès la première frame.
        let pending_click = Rc::new(Cell::new(origin));
        listen_pointer(&window, pointer.clone(), pending_click.clone())?;

        let anchor_nodes = document.query_selector_all("[data-gpu]")?;
        let anchors = (0..anchor_nodes.length())
            .filter_map(|index| anchor_nodes.item(index)?.dyn_into::<Element>().ok())
            .filter_map(|element| {
                let kind = match element.get_attribute("data-gpu")?.as_str() {
                    "panel" => KIND_PANEL,
                    "card" => KIND_CARD,
                    "title" => KIND_TITLE,
                    "target" => KIND_TARGET,
                    "reading" => KIND_READING,
                    "viz" => KIND_VIZ,
                    "scrim" => KIND_SCRIM,
                    _ => return None,
                };
                // Variante inconnue (faute de frappe dans l'article) : l'ancre est ignorée plutôt que mal dessinée.
                let variant = if kind == KIND_VIZ {
                    let name = element.get_attribute("data-viz")?;
                    VIZ_VARIANTS.iter().position(|variant| *variant == name)? as u32
                } else {
                    0
                };
                Some(Anchor { element, kind, glow: 0.0, variant })
            })
            .take(MAX_RECTS)
            .collect();

        Ok(Renderer {
            last_scroll: window.scroll_y()?,
            window,
            canvas,
            _instance: instance,
            device_lost,
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
            drawn_particles: particle_count,
            frame_ms: FRAME_BUDGET_MS,
            frames_rendered: 0,
            game,
            anchors,
            pointer,
            pending_click,
            shock: [0.0; 4],
            intensity: if mode.as_deref() == Some("calm") { CALM_INTENSITY } else { 1.0 },
            reduced_motion,
            last_time: 0.0,
            scroll_velocity: 0.0,
            last_pointer: POINTER_AWAY,
            pointer_velocity: [0.0; 2],
            fluid,
        })
    }

    fn frame(&mut self, now: f64) {
        let dpr = self.window.device_pixel_ratio().min(MAX_DPR);
        let width = self.window.inner_width().ok().and_then(|value| value.as_f64()).unwrap_or(1.0);
        let height = self.window.inner_height().ok().and_then(|value| value.as_f64()).unwrap_or(1.0);
        self.resize((width * dpr) as u32, (height * dpr) as u32);

        // dt borné : après un onglet caché, pas de saut de simulation.
        let dt = if self.last_time == 0.0 { 0.0 } else { ((now - self.last_time) / 1000.0).min(0.05) } as f32;
        self.frames_rendered = self.frames_rendered.saturating_add(1);
        if self.frames_rendered > WARMUP_FRAMES {
            self.adapt_quality((now - self.last_time) as f32);
        }
        self.last_time = now;
        let scroll = self.window.scroll_y().unwrap_or(0.0);
        let scroll_delta = (scroll - self.last_scroll) as f32;
        self.last_scroll = scroll;
        if dt > 0.0 {
            let easing = (dt * 10.0).min(1.0);
            self.scroll_velocity += (scroll_delta / dt - self.scroll_velocity) * easing;
        }

        let pointer = self.pointer.get();
        if dt > 0.0 {
            // Curseur qui entre ou sort de la page : pas de vitesse, sinon un saut géant agiterait le fluide.
            let target = if pointer == POINTER_AWAY || self.last_pointer == POINTER_AWAY {
                [0.0, 0.0]
            } else {
                [(pointer[0] - self.last_pointer[0]) / dt, (pointer[1] - self.last_pointer[1]) / dt]
            };
            let easing = (dt * 20.0).min(1.0);
            for axis in 0..2 {
                self.pointer_velocity[axis] += (target[axis] - self.pointer_velocity[axis]) * easing;
            }
        }
        self.last_pointer = pointer;
        let rects = self.collect_rects(pointer, dt);
        let reading_progress = reading_progress(&rects, height as f32);
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
            scroll_velocity: if frozen { 0.0 } else { self.scroll_velocity },
            reading_progress,
            shock: self.shock,
            pointer_velocity: if frozen { [0.0; 2] } else { self.pointer_velocity },
            fluid_grid: self.fluid.grid,
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
            if self.game.is_none() {
                self.fluid.encode(&mut pass);
            }
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
            pass.draw(0..6, 0..self.drawn_particles);
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

    /// Frames lentes en moyenne : un quart de particules dessinées en moins, sans descendre sous le quart du total.
    /// La mesure repart du budget d'une frame pour laisser le temps au changement de faire effet.
    fn adapt_quality(&mut self, frame_ms: f32) {
        // Un onglet caché peut donner des écarts énormes : on les borne pour ne pas fausser la moyenne.
        self.frame_ms += (frame_ms.min(100.0) - self.frame_ms) * 0.05;
        let floor = self.particle_count / 4;
        if self.frame_ms > SLOW_FRAME_MS && self.drawn_particles > floor {
            self.drawn_particles = (self.drawn_particles * 3 / 4).max(floor);
            self.frame_ms = FRAME_BUDGET_MS;
        }
    }

    /// Retour au rendu CSS seul : le canvas disparaît et la page retrouve ses fonds.
    fn shut_down(&self) {
        self.canvas.remove();
        if let Some(root) = self.window.document().and_then(|document| document.document_element()) {
            let _ = root.class_list().remove_1("gpu");
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
            let lit = match anchor.kind {
                KIND_CARD => (min[0]..max[0]).contains(&pointer[0]) && (min[1]..max[1]).contains(&pointer[1]),
                // Un titre s'allume (plus lentement) quand il entre dans le viewport.
                KIND_TITLE => max[1] > 0.0 && min[1] < viewport_height * 0.9,
                _ => false,
            };
            let rate = if anchor.kind == KIND_TITLE { easing * 0.3 } else { easing };
            anchor.glow += (f32::from(u8::from(lit)) - anchor.glow) * rate;
            // Hors écran (avec marge pour les halos), ou masqué en CSS (rectangle vide en 0,0) : rien à dessiner.
            let hidden = bounds.width() == 0.0 || bounds.height() == 0.0;
            if hidden || max[1] < -100.0 || min[1] > viewport_height + 100.0 {
                continue;
            }
            rects.push(Rect { min, max, kind: anchor.kind, glow: anchor.glow, variant: anchor.variant, _pad: 0.0 });
        }
        rects
    }
}

/// Avancée dans l'ancre `reading` : 0 quand son haut atteint le haut de l'écran, 1 quand son bas atteint le bas.
fn reading_progress(rects: &[Rect], viewport_height: f32) -> f32 {
    rects.iter().find(|rect| rect.kind == KIND_READING).map_or(0.0, |rect| {
        let scrollable = (rect.max[1] - rect.min[1] - viewport_height).max(1.0);
        (-rect.min[1] / scrollable).clamp(0.0, 1.0)
    })
}

fn listen_pointer(
    window: &Window,
    pointer: Rc<Cell<[f32; 2]>>,
    pending_click: Rc<Cell<Option<[f32; 2]>>>,
) -> Result<(), JsValue> {
    let storage = window.session_storage().ok().flatten();
    let on_down = Closure::<dyn FnMut(PointerEvent)>::new(move |event: PointerEvent| {
        let (x, y) = (event.client_x() as f32, event.client_y() as f32);
        pending_click.set(Some([x, y]));
        let on_link = event
            .target()
            .and_then(|target| target.dyn_into::<Element>().ok())
            .is_some_and(|element| element.closest("a[href]").ok().flatten().is_some());
        if let (true, Some(storage)) = (on_link, &storage) {
            // Stockage indisponible ou plein : pas de transition, rien de grave.
            let _ = storage.set_item(TRANSITION_KEY, &format!("{x},{y},{}", js_sys::Date::now()));
        }
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

/// Lit puis efface le point de clic mémorisé par la page précédente, s'il est récent.
fn take_transition_origin(window: &Window) -> Option<[f32; 2]> {
    let storage = window.session_storage().ok()??;
    let value = storage.get_item(TRANSITION_KEY).ok()??;
    let _ = storage.remove_item(TRANSITION_KEY);
    let parts: Vec<f64> = value.split(',').filter_map(|part| part.parse().ok()).collect();
    let [x, y, timestamp] = parts[..] else {
        return None;
    };
    (js_sys::Date::now() - timestamp < TRANSITION_MAX_AGE_MS).then_some([x as f32, y as f32])
}

/// Répartition pseudo-aléatoire (xorshift) : pas besoin d'une dépendance `rand` pour ça.
/// Avec `origin`, toutes les particules partent de ce point avec une vitesse sortante (transition d'entrée).
fn seed_particles(count: u32, width: f32, height: f32, slots: &[[f32; 2]], origin: Option<[f32; 2]>) -> Vec<Particle> {
    let mut state: u32 = 0x9e37_79b9;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        state as f32 / u32::MAX as f32
    };
    (0..count as usize)
        .map(|index| {
            let (pos, vel) = match origin {
                Some(origin) => {
                    let angle = next() * std::f32::consts::TAU;
                    let speed = BURST_MIN_SPEED + next() * (BURST_MAX_SPEED - BURST_MIN_SPEED);
                    (origin, [angle.cos() * speed, angle.sin() * speed])
                }
                None => ([next() * width, next() * height], [0.0, 0.0]),
            };
            Particle {
                pos,
                vel,
                // Pas premier : répartit les particules sur tout le masque plutôt que ligne par ligne.
                slot: slots[index * 7919 % slots.len()],
                captured: 0.0,
                _pad: 0.0,
            }
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
/// 3 : compteur de captures, 4 : vitesse du fluide (compute seulement).
fn bind_group_layout(device: &wgpu::Device, label: &str, compute: bool) -> wgpu::BindGroupLayout {
    let storage = |binding, read_only| wgpu::BindGroupLayoutEntry {
        binding,
        // Un storage inscriptible visible du vertex shader exigerait la feature VERTEX_WRITABLE_STORAGE.
        visibility: if read_only { VISIBLE_EVERYWHERE } else { wgpu::ShaderStages::COMPUTE },
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
            visibility: VISIBLE_EVERYWHERE,
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
        entries.push(storage(4, true));
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
