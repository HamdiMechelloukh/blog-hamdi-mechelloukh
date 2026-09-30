// Passe plein écran : fond nuit, panneaux sombres sous le contenu, halos et liserés autour des ancres.

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4f {
    // Triangle unique qui couvre tout l'écran.
    let uv = vec2f(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4f(uv * 2.0 - 1.0, 0.0, 1.0);
}

const NIGHT_TOP: vec3f = vec3f(0.020, 0.024, 0.047);
const NIGHT_BOTTOM: vec3f = vec3f(0.047, 0.027, 0.063);
const PANEL: vec3f = vec3f(0.043, 0.051, 0.078);

// Tout texte posé sur le fond (titres, descriptions, en-têtes) : fond assombri en fondu autour, sans panneau.
fn darken_behind(color: vec3f, p: vec2f, rect: Rect) -> vec3f {
    let d = rect_distance(p, rect, 0.0, 12.0);
    return color * (1.0 - 0.8 * (1.0 - smoothstep(-8.0, 40.0, d)));
}

@fragment
fn fs_main(@builtin(position) frag: vec4f) -> @location(0) vec4f {
    // L'onde de choc déforme le fond : on échantillonne la scène un peu en retrait de l'anneau.
    let ring = shock_ring(frag.xy / globals.dpr);
    let p = frag.xy / globals.dpr - ring.xy * ring.z * 14.0;
    let uv = p / globals.resolution;

    var color = mix(NIGHT_TOP, NIGHT_BOTTOM, uv.y);
    // Lueur lointaine qui suit lentement le curseur.
    let pointer_glow = exp(-length(p - globals.pointer) / 380.0);
    color += ACCENT * pointer_glow * 0.06;

    for (var i = 0u; i < globals.rect_count; i++) {
        let rect = rects[i];
        if (rect.kind == KIND_TARGET) {
            continue;
        }
        if (rect.kind == KIND_SCRIM) {
            color = darken_behind(color, p, rect);
            continue;
        }
        if (rect.kind == KIND_VIZ) {
            let d = rect_distance(p, rect, 0.0, 14.0);
            color = mix(color, PANEL, (1.0 - smoothstep(-1.0, 1.0, d)) * 0.94);
            if (d < 0.0) {
                color += viz(rect.variant, p - rect.min, rect.max - rect.min, globals.time);
            }
            color += ACCENT * (1.0 - smoothstep(0.0, 1.5, abs(d))) * 0.25;
            continue;
        }
        if (rect.kind == KIND_TITLE) {
            color = darken_behind(color, p, rect);
            let d = rect_distance(p, rect, 12.0, 24.0);
            let shimmer = 0.75 + 0.25 * sin(globals.time * 1.3 + p.x * 0.012);
            // Le titre s'allume en entrant à l'écran (glow), avec un éclat au moment où il s'allume.
            let ignition = rect.glow * (1.0 + 1.5 * rect.glow * (1.0 - rect.glow));
            // Halo seulement autour du texte : dessous, le fond reste sombre pour le contraste.
            let outside = smoothstep(0.0, 12.0, d);
            color += ACCENT * exp(-max(d, 0.0) / 36.0) * 0.16 * shimmer * ignition * outside;
            continue;
        }
        let is_panel = rect.kind == KIND_PANEL || rect.kind == KIND_READING;
        let radius = select(12.0, 16.0, is_panel);
        let d = rect_distance(p, rect, 0.0, radius);
        // Panneau : les lectures longues sont quasi opaques pour garantir le contraste.
        let opacity = select(0.82, 0.96, is_panel);
        color = mix(color, PANEL, (1.0 - smoothstep(-1.0, 1.0, d)) * opacity);
        var glow = rect.glow;
        if (rect.kind == KIND_READING) {
            // Liseré rempli du haut jusqu'au point de lecture.
            let depth = (p.y - rect.min.y) / max(rect.max.y - rect.min.y, 1.0);
            glow = 1.0 - smoothstep(globals.reading_progress - 0.01, globals.reading_progress, depth);
        }
        let halo = exp(-max(d, 0.0) / (14.0 + 18.0 * glow)) * (0.05 + 0.30 * glow);
        let edge = (1.0 - smoothstep(0.0, 1.5, abs(d))) * (0.18 + 0.62 * glow);
        color += mix(ACCENT, ACCENT_HOT, glow) * (halo + edge);
    }

    color += ACCENT_HOT * ring.z * 0.10;

    // Vignette.
    let centered = uv - 0.5;
    color *= 1.0 - dot(centered, centered) * 0.6;
    return vec4f(color, 1.0);
}
