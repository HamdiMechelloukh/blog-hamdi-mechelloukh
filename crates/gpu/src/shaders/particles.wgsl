// Rendu des particules : un quad par instance, disque doux en blending additif.

@group(0) @binding(2) var<storage, read> particles: array<Particle>;

struct VertexOut {
    @builtin(position) position: vec4f,
    @location(0) local: vec2f,
    @location(1) color: vec3f,
}

const CORNERS: array<vec2f, 6> = array<vec2f, 6>(
    vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(-1.0, 1.0),
    vec2f(-1.0, 1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0),
);
const RADIUS: f32 = 1.6;

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> VertexOut {
    let particle = particles[instance];
    let corner = CORNERS[vertex];

    // Atténuation au-dessus des ancres : le texte reste lisible même si une particule s'y aventure.
    var fade = 1.0;
    for (var i = 0u; i < globals.rect_count; i++) {
        fade = min(fade, smoothstep(-4.0, 10.0, rect_distance(particle.pos, rects[i], 0.0, 12.0)));
    }

    let speed = clamp(length(particle.vel) / 60.0, 0.0, 1.0);
    let brightness = (0.10 + 0.25 * speed) * fade * (0.35 + 0.65 * globals.intensity);

    let pixel = particle.pos + corner * RADIUS;
    let ndc = pixel / globals.resolution * vec2f(2.0, -2.0) + vec2f(-1.0, 1.0);
    var out: VertexOut;
    out.position = vec4f(ndc, 0.0, 1.0);
    out.local = corner;
    out.color = mix(ACCENT, ACCENT_HOT, speed) * brightness;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4f {
    let falloff = max(1.0 - dot(in.local, in.local), 0.0);
    return vec4f(in.color * falloff * falloff, 1.0);
}
