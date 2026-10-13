//! Page /jeu : combat automatique de robots programmés à l'avance contre une IA.
//! La simulation (Rust pur, déterministe) est séparée du rendu : `cargo test` la couvre en natif.

pub mod ai;
pub mod loadout;
pub mod program;
pub mod rng;
pub mod sim;
pub mod vec2;

#[cfg(target_arch = "wasm32")]
mod render;
#[cfg(target_arch = "wasm32")]
mod web;
