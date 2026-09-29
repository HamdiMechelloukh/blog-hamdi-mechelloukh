// Structures partagées par tous les shaders. Toutes les coordonnées sont en pixels CSS,
// origine en haut à gauche du viewport (comme getBoundingClientRect).

struct Globals {
    resolution: vec2f,
    pointer: vec2f,
    time: f32,
    dt: f32,
    scroll_delta: f32,
    // 1.0 sur les pages vitrines, faible sur les articles (data-gpu-mode="calm").
    intensity: f32,
    rect_count: u32,
    dpr: f32,
    _pad: vec2f,
}

// Ancre DOM (data-gpu) projetée dans le viewport.
struct Rect {
    min: vec2f,
    max: vec2f,
    kind: u32,
    // 0 -> 1, lissé côté Rust quand le curseur survole l'ancre.
    glow: f32,
    _pad: vec2f,
}

struct Particle {
    pos: vec2f,
    vel: vec2f,
    // Page 404 : place dans le masque du « 404 » (0..1 dans l'ancre target), et 1.0 une fois capturée.
    slot: vec2f,
    captured: f32,
    _pad: f32,
}

const KIND_PANEL: u32 = 0u;
const KIND_CARD: u32 = 1u;
const KIND_TITLE: u32 = 2u;
// Zone où se reforme le « 404 » : ni panneau ni obstacle.
const KIND_TARGET: u32 = 3u;
const TAU: f32 = 6.2831853;

const ACCENT: vec3f = vec3f(0.976, 0.451, 0.086); // #f97316, l'orange de la charte
const ACCENT_HOT: vec3f = vec3f(1.0, 0.72, 0.35);

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<storage, read> rects: array<Rect>;

// Distance signée à une boîte arrondie centrée en 0 : négative à l'intérieur.
fn sd_round_box(p: vec2f, half_size: vec2f, radius: f32) -> f32 {
    let q = abs(p) - half_size + radius;
    return length(max(q, vec2f(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

fn flow(p: vec2f, t: f32) -> vec2f {
    let angle = (sin(p.x * 0.0041 + t * 0.11) * cos(p.y * 0.0053 - t * 0.07)
        + 0.5 * sin((p.x + p.y) * 0.0021 + t * 0.05)) * TAU;
    return vec2f(cos(angle), sin(angle));
}

// Sortie d'écran : réapparition du côté opposé.
fn wrap(p: vec2f) -> vec2f {
    return p - floor(p / globals.resolution) * globals.resolution;
}

fn rect_distance(p: vec2f, rect: Rect, margin: f32, radius: f32) -> f32 {
    let center = (rect.min + rect.max) * 0.5;
    let half_size = (rect.max - rect.min) * 0.5 + margin;
    return sd_round_box(p - center, half_size, radius);
}
