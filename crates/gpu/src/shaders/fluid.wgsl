// Fluide incompressible (stable fluids, Jos Stam) sur une grille fixe : advection semi-lagrangienne,
// forces (curseur, bruit ambiant), divergence, pression par Jacobi, projection.
// Pas d'onde de choc ici : une poussée radiale est une dilatation pure, que la projection annulerait.
// Elle agit directement sur les particules (compute.wgsl).
// Cellules éventuellement non carrées : cell = resolution / grid, pas distincts en x et en y.
// Bindings 2 à 6 : 0 est globals (common.wgsl), 1 est réservé aux ancres.

@group(0) @binding(2) var<storage, read> velocity_in: array<vec2f>;
@group(0) @binding(3) var<storage, read_write> velocity_out: array<vec2f>;
@group(0) @binding(4) var<storage, read> pressure_in: array<f32>;
@group(0) @binding(5) var<storage, read_write> pressure_out: array<f32>;
@group(0) @binding(6) var<storage, read_write> divergence: array<f32>;

const POINTER_RADIUS: f32 = 70.0;
const POINTER_STRENGTH: f32 = 0.6;
const AMBIENT_FORCE: f32 = 45.0;
const DISSIPATION: f32 = 0.35;
// Maelstrom : force tangentielle en anneau autour du curseur (nulle au centre, éteinte au-delà de 150 px).
// Champ purement rotatif, donc sans divergence : la projection le conserve, contrairement à une poussée radiale.
const VORTEX_FORCE: f32 = 260.0;
const VORTEX_INNER: f32 = 40.0;
const VORTEX_OUTER: f32 = 150.0;
const MAX_FLUID_SPEED: f32 = 1500.0;

fn grid() -> vec2i {
    return vec2i(globals.fluid_grid);
}

fn cell_size() -> vec2f {
    return globals.resolution / vec2f(globals.fluid_grid);
}

// Indice d'une cellule, bornée à la grille (bords : on recopie la cellule voisine).
fn index(cell: vec2i) -> u32 {
    let clamped = clamp(cell, vec2i(0), grid() - 1);
    return u32(clamped.y * grid().x + clamped.x);
}

// Vitesse interpolée (bilinéaire) en un point exprimé en cellules, centres en +0.5.
fn sample_velocity(position: vec2f) -> vec2f {
    let p = position - 0.5;
    let base = vec2i(floor(p));
    let t = fract(p);
    let bottom = mix(velocity_in[index(base)], velocity_in[index(base + vec2i(1, 0))], t.x);
    let top = mix(velocity_in[index(base + vec2i(0, 1))], velocity_in[index(base + vec2i(1, 1))], t.x);
    return mix(bottom, top, t.y);
}

fn in_grid(id: vec3u) -> bool {
    return all(vec2i(id.xy) < grid());
}

@compute @workgroup_size(8, 8)
fn advect(@builtin(global_invocation_id) id: vec3u) {
    if (!in_grid(id)) {
        return;
    }
    let cell = vec2i(id.xy);
    let size = cell_size();
    let center = (vec2f(cell) + 0.5) * size;
    let dt = globals.dt;

    // Semi-lagrangien : on va chercher la vitesse là d'où vient le fluide.
    let here = velocity_in[index(cell)];
    var velocity = sample_velocity((center - here * dt) / size) * pow(DISSIPATION, dt);

    let to_pointer = center - globals.pointer;
    let pointer_weight = exp(-dot(to_pointer, to_pointer) / (POINTER_RADIUS * POINTER_RADIUS));
    // Le fluide sous le curseur prend sa vitesse (mélange, pas addition : pas d'accumulation d'une frame à l'autre),
    // seulement quand le curseur bouge : immobile, il ne freine pas le fluide.
    let moving = min(length(globals.pointer_velocity) / 50.0, 1.0);
    velocity = mix(velocity, globals.pointer_velocity, pointer_weight * POINTER_STRENGTH * moving);
    velocity += flow(center, globals.time) * AMBIENT_FORCE * dt;
    let pointer_distance = length(to_pointer);
    if (globals.vortex > 0.0 && pointer_distance > 1.0) {
        let tangent = vec2f(-to_pointer.y, to_pointer.x) / pointer_distance;
        let ring = smoothstep(0.0, VORTEX_INNER, pointer_distance) * (1.0 - smoothstep(90.0, VORTEX_OUTER, pointer_distance));
        velocity += tangent * ring * VORTEX_FORCE * globals.vortex * dt;
    }

    let speed = length(velocity);
    if (speed > MAX_FLUID_SPEED) {
        velocity *= MAX_FLUID_SPEED / speed;
    }
    velocity_out[index(cell)] = velocity;
}

@compute @workgroup_size(8, 8)
fn compute_divergence(@builtin(global_invocation_id) id: vec3u) {
    if (!in_grid(id)) {
        return;
    }
    let cell = vec2i(id.xy);
    let size = cell_size();
    let left = velocity_in[index(cell - vec2i(1, 0))].x;
    let right = velocity_in[index(cell + vec2i(1, 0))].x;
    let down = velocity_in[index(cell - vec2i(0, 1))].y;
    let up = velocity_in[index(cell + vec2i(0, 1))].y;
    divergence[index(cell)] = (right - left) / (2.0 * size.x) + (up - down) / (2.0 * size.y);
}

// Une itération de Jacobi pour l'équation de Poisson (laplacien de p = divergence), pas anisotropes.
@compute @workgroup_size(8, 8)
fn jacobi(@builtin(global_invocation_id) id: vec3u) {
    if (!in_grid(id)) {
        return;
    }
    let cell = vec2i(id.xy);
    let inverse_x = 1.0 / (cell_size().x * cell_size().x);
    let inverse_y = 1.0 / (cell_size().y * cell_size().y);
    let horizontal = pressure_in[index(cell - vec2i(1, 0))] + pressure_in[index(cell + vec2i(1, 0))];
    let vertical = pressure_in[index(cell - vec2i(0, 1))] + pressure_in[index(cell + vec2i(0, 1))];
    pressure_out[index(cell)] =
        (horizontal * inverse_x + vertical * inverse_y - divergence[index(cell)]) / (2.0 * (inverse_x + inverse_y));
}

// Retire le gradient de pression : le champ devient incompressible, d'où les tourbillons.
@compute @workgroup_size(8, 8)
fn project(@builtin(global_invocation_id) id: vec3u) {
    if (!in_grid(id)) {
        return;
    }
    let cell = vec2i(id.xy);
    let size = cell_size();
    let gradient = vec2f(
        (pressure_in[index(cell + vec2i(1, 0))] - pressure_in[index(cell - vec2i(1, 0))]) / (2.0 * size.x),
        (pressure_in[index(cell + vec2i(0, 1))] - pressure_in[index(cell - vec2i(0, 1))]) / (2.0 * size.y),
    );
    velocity_out[index(cell)] = velocity_in[index(cell)] - gradient;
}
