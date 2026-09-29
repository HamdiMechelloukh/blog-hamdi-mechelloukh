// Simulation des particules : flow field + contournement des ancres DOM + attraction du curseur.

@group(0) @binding(2) var<storage, read_write> particles: array<Particle>;
// Champ de vitesse du fluide (fluid.wgsl), une vitesse par cellule.
@group(0) @binding(4) var<storage, read> fluid_velocity: array<vec2f>;

const REPEL_MARGIN: f32 = 28.0;
const MAX_SPEED: f32 = 1500.0;
// Vitesse à laquelle une particule adopte la vitesse du fluide (1/s).
const FLUID_COUPLING: f32 = 2.5;

// Vitesse du fluide au point p (px CSS), interpolée entre les centres de cellules.
fn fluid_at(p: vec2f) -> vec2f {
    let grid = vec2i(globals.fluid_grid);
    let position = p / globals.resolution * vec2f(globals.fluid_grid) - 0.5;
    let base = vec2i(floor(position));
    let t = fract(position);
    let a = clamp(base, vec2i(0), grid - 1);
    let b = clamp(base + 1, vec2i(0), grid - 1);
    let width = u32(grid.x);
    let bottom = mix(fluid_velocity[u32(a.y) * width + u32(a.x)], fluid_velocity[u32(a.y) * width + u32(b.x)], t.x);
    let top = mix(fluid_velocity[u32(b.y) * width + u32(a.x)], fluid_velocity[u32(b.y) * width + u32(b.x)], t.x);
    return mix(bottom, top, t.y);
}

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

    // Le fluide porte le mouvement ambiant et le sillage du curseur ; restent ici les forces propres aux particules.
    // L'onde de choc en fait partie : incompressible, le fluide annulerait cette poussée radiale.
    let ring = shock_ring(particle.pos);
    var force = ring.xy * ring.z * 2600.0;
    for (var i = 0u; i < globals.rect_count; i++) {
        force += repel(particle.pos, rects[i]) * 60.0;
    }
    let to_pointer = globals.pointer - particle.pos;
    let pointer_distance = length(to_pointer);
    if (pointer_distance < 220.0 && pointer_distance > 1.0) {
        // Attraction, qui s'inverse près du curseur : un anneau plutôt qu'un amas. Des milliers de particules
        // superposées sur les mêmes pixels saturent le blending et peuvent faire décrocher un GPU intégré.
        let pull = (1.0 - pointer_distance / 220.0) * 90.0;
        let push = (1.0 - smoothstep(0.0, 50.0, pointer_distance)) * 260.0;
        force += to_pointer / pointer_distance * (pull - push);
    }

    particle.vel += force * dt * globals.intensity;
    particle.vel = mix(particle.vel, fluid_at(particle.pos), min(dt * FLUID_COUPLING, 1.0));
    // Vitesse plafonnée : bornes la longueur des traînées (et donc le coût de rendu) après une onde de choc.
    let speed = length(particle.vel);
    if (speed > MAX_SPEED) {
        particle.vel *= MAX_SPEED / speed;
    }
    particle.pos += particle.vel * dt * globals.intensity;

    particle.pos = wrap(particle.pos);
    particles[index] = particle;
}
