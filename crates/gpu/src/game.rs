//! Page 404 (`data-gpu-mode="game"`) : le curseur capture les particules, qui reforment le « 404 ».
//! Le compute shader compte les captures dans un compteur atomique, relu ici de façon asynchrone.

use std::cell::Cell;
use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{CanvasRenderingContext2d, Document, Element, HtmlCanvasElement};

pub const PARTICLES: u32 = 8_000;
/// Part des particules à capturer pour gagner.
const WIN_RATIO: f32 = 0.8;
/// Masque du texte : même proportion (5:2) que l'ancre `.game-target` en CSS.
const MASK_WIDTH: u32 = 500;
const MASK_HEIGHT: u32 = 200;
const MASK_STEP: usize = 2;
/// Relecture du compteur toutes les N frames : inutile de la faire à chaque image.
const READBACK_INTERVAL: u32 = 10;

/// Places des particules dans le « 404 », en coordonnées normalisées (0..1) de l'ancre cible.
pub async fn glyph_slots(document: &Document) -> Result<Vec<[f32; 2]>, JsValue> {
    // La police du site doit être chargée pour que le « 404 » ait la bonne forme.
    JsFuture::from(document.fonts().ready()?).await?;
    let canvas: HtmlCanvasElement = document.create_element("canvas")?.dyn_into()?;
    canvas.set_width(MASK_WIDTH);
    canvas.set_height(MASK_HEIGHT);
    let context: CanvasRenderingContext2d =
        canvas.get_context("2d")?.ok_or("contexte 2D indisponible")?.dyn_into()?;
    context.set_font("700 190px Outfit, sans-serif");
    context.set_text_align("center");
    context.set_text_baseline("middle");
    context.fill_text("404", f64::from(MASK_WIDTH) / 2.0, f64::from(MASK_HEIGHT) / 2.0 + 8.0)?;
    let pixels = context.get_image_data(0.0, 0.0, f64::from(MASK_WIDTH), f64::from(MASK_HEIGHT))?.data();

    let (width, height) = (MASK_WIDTH as usize, MASK_HEIGHT as usize);
    let mut slots = Vec::new();
    for y in (0..height).step_by(MASK_STEP) {
        for x in (0..width).step_by(MASK_STEP) {
            let alpha = pixels[(y * width + x) * 4 + 3];
            if alpha > 127 {
                slots.push([x as f32 / width as f32, y as f32 / height as f32]);
            }
        }
    }
    if slots.is_empty() {
        return Err("masque du 404 vide".into());
    }
    Ok(slots)
}

pub struct Game {
    pub pipeline: wgpu::ComputePipeline,
    counter: wgpu::Buffer,
    readback: wgpu::Buffer,
    captured: Rc<Cell<u32>>,
    readback_in_flight: Rc<Cell<bool>>,
    frames_since_readback: u32,
    progress: Option<Element>,
    root: Element,
    goal: u32,
    won: bool,
}

impl Game {
    pub fn new(device: &wgpu::Device, pipeline: wgpu::ComputePipeline, counter: wgpu::Buffer, document: &Document) -> Result<Self, JsValue> {
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("captured readback"),
            size: 4,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Ok(Game {
            pipeline,
            counter,
            readback,
            captured: Rc::new(Cell::new(0)),
            readback_in_flight: Rc::new(Cell::new(false)),
            frames_since_readback: 0,
            progress: document.get_element_by_id("game-progress"),
            root: document.document_element().ok_or("pas de <html>")?,
            goal: (PARTICLES as f32 * WIN_RATIO) as u32,
            won: false,
        })
    }

    /// À appeler avant `queue.submit` : copie le compteur si aucune relecture n'est en cours.
    /// Renvoie true si une copie a été encodée.
    pub fn encode_readback(&mut self, encoder: &mut wgpu::CommandEncoder) -> bool {
        self.frames_since_readback += 1;
        if self.won || self.readback_in_flight.get() || self.frames_since_readback < READBACK_INTERVAL {
            return false;
        }
        self.frames_since_readback = 0;
        encoder.copy_buffer_to_buffer(&self.counter, 0, &self.readback, 0, 4);
        true
    }

    /// À appeler après `queue.submit` quand `encode_readback` a renvoyé true.
    pub fn start_readback(&self) {
        self.readback_in_flight.set(true);
        let (readback, captured, in_flight) =
            (self.readback.clone(), self.captured.clone(), self.readback_in_flight.clone());
        self.readback.map_async(wgpu::MapMode::Read, .., move |result| {
            if result.is_ok() {
                if let Ok(view) = readback.get_mapped_range(..) {
                    captured.set(u32::from_le_bytes([view[0], view[1], view[2], view[3]]));
                }
                readback.unmap();
            }
            in_flight.set(false);
        });
    }

    /// Met à jour la progression affichée et déclare la victoire.
    pub fn update_dom(&mut self) {
        if self.won {
            return;
        }
        let percent = (self.captured.get() * 100 / self.goal).min(100);
        if let Some(progress) = &self.progress {
            progress.set_text_content(Some(&format!("{percent} %")));
        }
        if percent == 100 {
            self.won = true;
            let _ = self.root.class_list().add_1("game-complete");
        }
    }
}
