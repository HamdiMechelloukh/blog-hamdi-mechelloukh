// Bloom : extraction des zones lumineuses, flou gaussien séparable, composition dans le canvas.
// Pas d'uniform : chaque passe lit la taille de sa texture source avec textureDimensions.

@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var linear_sampler: sampler;
// Composition seulement : le halo flouté, ajouté à l'image HDR (source).
@group(0) @binding(2) var bloom: texture_2d<f32>;

// Au-dessus de ce niveau, la lumière déborde.
const THRESHOLD: f32 = 0.55;
const STRENGTH: f32 = 0.9;
// Gaussienne 9 taps, échantillonnée en 5 lectures grâce au filtrage bilinéaire.
const OFFSETS: array<f32, 3> = array<f32, 3>(0.0, 1.3846153846, 3.2307692308);
const WEIGHTS: array<f32, 3> = array<f32, 3>(0.2270270270, 0.3162162162, 0.0702702703);

struct VertexOut {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOut {
    let corner = vec2f(f32((index << 1u) & 2u), f32(index & 2u));
    var out: VertexOut;
    out.position = vec4f(corner * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2f(corner.x, 1.0 - corner.y);
    return out;
}

@fragment
fn fs_bright(in: VertexOut) -> @location(0) vec4f {
    let color = textureSample(source, linear_sampler, in.uv).rgb;
    let luminance = dot(color, vec3f(0.2126, 0.7152, 0.0722));
    // Seuil doux : pas de frontière nette entre ce qui brille et le reste.
    let excess = max(luminance - THRESHOLD, 0.0);
    return vec4f(color * excess / max(luminance, 1e-4), 1.0);
}

fn blur(uv: vec2f, direction: vec2f) -> vec4f {
    let texel = direction / vec2f(textureDimensions(source));
    var color = textureSample(source, linear_sampler, uv).rgb * WEIGHTS[0];
    for (var i = 1; i < 3; i++) {
        color += textureSample(source, linear_sampler, uv + texel * OFFSETS[i]).rgb * WEIGHTS[i];
        color += textureSample(source, linear_sampler, uv - texel * OFFSETS[i]).rgb * WEIGHTS[i];
    }
    return vec4f(color, 1.0);
}

@fragment
fn fs_blur_horizontal(in: VertexOut) -> @location(0) vec4f {
    return blur(in.uv, vec2f(1.0, 0.0));
}

@fragment
fn fs_blur_vertical(in: VertexOut) -> @location(0) vec4f {
    return blur(in.uv, vec2f(0.0, 1.0));
}

@fragment
fn fs_composite(in: VertexOut) -> @location(0) vec4f {
    let hdr = textureSample(source, linear_sampler, in.uv).rgb;
    let glow = textureSample(bloom, linear_sampler, in.uv).rgb;
    return vec4f(min(hdr + glow * STRENGTH, vec3f(1.0)), 1.0);
}
