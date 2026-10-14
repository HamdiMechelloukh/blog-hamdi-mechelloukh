// Arène : chaque forme est une instance (quad) dessinée par sa fonction de distance signée,
// en contour lumineux avec halo, mélangée en additif.

struct Globals {
    arena: vec2f,
    // Taille d'un pixel de l'écran en unités d'arène : anticrénelage sans fwidth.
    pixel: f32,
    time: f32,
}

@group(0) @binding(0) var<uniform> globals: Globals;

struct Instance {
    @location(0) center: vec2f,
    @location(1) half_size: vec2f,
    @location(2) color: vec4f,
    // x : angle, y : forme, z et w : paramètres de la forme (voir SHAPE_*).
    @location(3) params: vec4f,
}

struct VertexOut {
    @builtin(position) position: vec4f,
    @location(0) local: vec2f,
    @location(1) half_size: vec2f,
    @location(2) color: vec4f,
    @location(3) params: vec4f,
}

// Fond de l'arène : grille et bordure.
const SHAPE_ARENA: f32 = 0.0;
// Disque plein de rayon half_size.x.
const SHAPE_DISC: f32 = 1.0;
// Anneau de rayon half_size.x, demi-épaisseur z.
const SHAPE_RING: f32 = 2.0;
// Rectangle arrondi (coin z) en contour, rempli à hauteur de w.
const SHAPE_BOX: f32 = 3.0;

const GLOW_WIDTH: f32 = 6.0;
const GLOW_STRENGTH: f32 = 0.55;
// Le quad déborde de la forme pour laisser la place au halo.
const GLOW_MARGIN: f32 = 18.0;
const GRID: f32 = 50.0;
const STROKE: f32 = 1.0;

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, instance: Instance) -> VertexOut {
    let corner = vec2f(f32(vertex & 1u), f32(vertex >> 1u)) * 2.0 - 1.0;
    let margin = select(GLOW_MARGIN, 0.0, instance.params.y == SHAPE_ARENA);
    let local = corner * (instance.half_size + vec2f(margin));
    let angle = instance.params.x;
    let rotated = vec2f(
        local.x * cos(angle) - local.y * sin(angle),
        local.x * sin(angle) + local.y * cos(angle),
    );
    let world = instance.center + rotated;
    let clip = world / globals.arena * 2.0 - 1.0;

    var out: VertexOut;
    out.position = vec4f(clip.x, -clip.y, 0.0, 1.0);
    out.local = local;
    out.half_size = instance.half_size;
    out.color = instance.color;
    out.params = instance.params;
    return out;
}

fn sd_box(p: vec2f, half_size: vec2f, radius: f32) -> f32 {
    let q = abs(p) - half_size + vec2f(radius);
    return length(max(q, vec2f(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4f {
    let p = in.local;
    let shape = in.params.y;
    let aa = globals.pixel;

    let box_distance = sd_box(p, in.half_size, in.params.z);
    var distance = abs(box_distance) - STROKE;
    var fill = select(0.0, in.params.w, box_distance < 0.0);
    if shape == SHAPE_DISC {
        distance = length(p) - in.half_size.x;
        fill = 0.0;
    } else if shape == SHAPE_RING {
        distance = abs(length(p) - in.half_size.x) - in.params.z;
        fill = 0.0;
    }

    let core = 1.0 - smoothstep(-aa, aa, distance);
    // Le halo s'éteint avant le bord du quad, sinon on en verrait le carré.
    let fade = 1.0 - smoothstep(GLOW_MARGIN * 0.5, GLOW_MARGIN, distance);
    let glow = exp(-max(distance, 0.0) / GLOW_WIDTH) * GLOW_STRENGTH * fade;
    var intensity = max(core, glow) + fill;
    if shape == SHAPE_ARENA {
        let cell = abs(fract(p / GRID + 0.5) - 0.5) * GRID;
        let line = 1.0 - smoothstep(0.0, aa * 1.5, min(cell.x, cell.y));
        intensity = max(core, glow * 0.5) + line * 0.06;
    }
    return vec4f(in.color.rgb * intensity * in.color.a, 1.0);
}
