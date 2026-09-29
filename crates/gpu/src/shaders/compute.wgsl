// Simulation des particules : flow field + contournement des ancres DOM + attraction du curseur.

@group(0) @binding(2) var<storage, read_write> particles: array<Particle>;

const REPEL_MARGIN: f32 = 28.0;

// Pousse la particule hors de la boîte (marge incluse), le long de la normale sortante.
fn repel(p: vec2f, rect: Rect) -> vec2f {
    if (rect.kind == KIND_TARGET) {
        return vec2f(0.0);
    }
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

    particle.pos = wrap(particle.pos);
    particles[index] = particle;
}
