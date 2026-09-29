// Simulation des particules : flow field + contournement des ancres DOM + attraction du curseur.

@group(0) @binding(2) var<storage, read_write> particles: array<Particle>;

const TAU: f32 = 6.2831853;
const REPEL_MARGIN: f32 = 28.0;

fn flow(p: vec2f, t: f32) -> vec2f {
    let angle = (sin(p.x * 0.0041 + t * 0.11) * cos(p.y * 0.0053 - t * 0.07)
        + 0.5 * sin((p.x + p.y) * 0.0021 + t * 0.05)) * TAU;
    return vec2f(cos(angle), sin(angle));
}

// Pousse la particule hors de la boîte (marge incluse), le long de la normale sortante.
fn repel(p: vec2f, rect: Rect) -> vec2f {
    let d = rect_distance(p, rect, REPEL_MARGIN, 0.0);
    if (d >= 0.0) {
        return vec2f(0.0);
    }
    let center = (rect.min + rect.max) * 0.5;
    let q = abs(p - center) - (rect.max - rect.min) * 0.5 - REPEL_MARGIN;
    let side = sign(p - center + vec2f(1e-3));
    let normal = select(vec2f(0.0, side.y), vec2f(side.x, 0.0), q.x > q.y);
    return normal * (-d) * 0.9;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3u) {
    let index = id.x;
    if (index >= arrayLength(&particles)) {
        return;
    }
    var particle = particles[index];
    let dt = globals.dt;

    // Parallaxe : le champ glisse moins vite que la page.
    particle.pos.y -= globals.scroll_delta * 0.35;

    var force = flow(particle.pos, globals.time) * 22.0;
    for (var i = 0u; i < globals.rect_count; i++) {
        force += repel(particle.pos, rects[i]) * 60.0;
    }
    let to_pointer = globals.pointer - particle.pos;
    let pointer_distance = length(to_pointer);
    if (pointer_distance < 220.0 && pointer_distance > 1.0) {
        force += to_pointer / pointer_distance * (1.0 - pointer_distance / 220.0) * 90.0;
    }

    particle.vel = (particle.vel + force * dt * globals.intensity) * pow(0.12, dt);
    particle.pos += particle.vel * dt * globals.intensity;

    // Sortie d'écran : réapparition du côté opposé.
    let size = globals.resolution;
    particle.pos = particle.pos - floor(particle.pos / size) * size;
    particles[index] = particle;
}
