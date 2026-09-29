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
    // Vitesse de défilement lissée (px/s) : étire les particules en traînées.
    scroll_velocity: f32,
    // 0 -> 1 : avancée dans le texte de l'article (ancre reading).
    reading_progress: f32,
    // Dernier clic : position (px CSS), instant (s, même base que time), 1.0 si actif.
    shock: vec4f,
    // Vitesse du curseur (px/s, lissée) : entraîne le fluide.
    pointer_velocity: vec2f,
    // Taille de la grille du fluide, en cellules.
    fluid_grid: vec2u,
}

// Ancre DOM (data-gpu) projetée dans le viewport.
struct Rect {
    min: vec2f,
    max: vec2f,
    kind: u32,
    // 0 -> 1, lissé côté Rust : survol pour les cartes, entrée à l'écran pour les titres.
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
// Texte d'un article : panneau dont le liseré suit la progression de lecture.
const KIND_READING: u32 = 4u;
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

const SHOCK_SPEED: f32 = 900.0;
const SHOCK_WIDTH: f32 = 70.0;
const SHOCK_DURATION: f32 = 1.4;

// Anneau de l'onde de choc au point p : xy = direction sortante, z = intensité (0 hors de l'anneau).
fn shock_ring(p: vec2f) -> vec3f {
    let age = globals.time - globals.shock.z;
    if (globals.shock.w < 0.5 || age < 0.0 || age > SHOCK_DURATION) {
        return vec3f(0.0);
    }
    let offset = p - globals.shock.xy;
    let distance = length(offset);
    let band = 1.0 - smoothstep(0.0, SHOCK_WIDTH, abs(distance - age * SHOCK_SPEED));
    let fade = 1.0 - age / SHOCK_DURATION;
    return vec3f(offset / max(distance, 1.0), band * fade * fade);
}

fn rect_distance(p: vec2f, rect: Rect, margin: f32, radius: f32) -> f32 {
    let center = (rect.min + rect.max) * 0.5;
    let half_size = (rect.max - rect.min) * 0.5 + margin;
    return sd_round_box(p - center, half_size, radius);
}
