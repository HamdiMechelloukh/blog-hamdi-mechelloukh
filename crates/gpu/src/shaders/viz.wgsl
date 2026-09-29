// Visualisations des articles (ancres data-gpu="viz", data-viz=<variante>), dessinées par scene.wgsl dans leur rectangle.
// p : pixel relatif au coin haut gauche de l'ancre, size : taille de l'ancre (px CSS), t : temps (s).
// Les libellés sont en HTML, positionnés en % aux mêmes coordonnées normalisées que les nœuds ci-dessous.

const VIZ_FLINK: u32 = 0u;
const VIZ_CONDORCET: u32 = 1u;
const VIZ_AGENTS: u32 = 2u;
const VIZ_LAKEHOUSE: u32 = 3u;

const RED: vec3f = vec3f(0.95, 0.25, 0.22);
const GREEN: vec3f = vec3f(0.30, 0.90, 0.50);
const NEUTRAL: vec3f = vec3f(0.55, 0.58, 0.70);
const BRONZE: vec3f = vec3f(0.80, 0.48, 0.20);
const SILVER: vec3f = vec3f(0.72, 0.78, 0.90);
const GOLD: vec3f = vec3f(1.00, 0.80, 0.30);
const NODE_RADIUS: f32 = 9.0;

fn hash(n: f32) -> f32 {
    return fract(sin(n * 127.1) * 43758.5453);
}

fn dot_glow(p: vec2f, center: vec2f, radius: f32) -> f32 {
    let d = p - center;
    return exp(-dot(d, d) / (radius * radius));
}

fn segment_distance(p: vec2f, a: vec2f, b: vec2f) -> f32 {
    let ab = b - a;
    let h = clamp(dot(p - a, ab) / dot(ab, ab), 0.0, 1.0);
    return length(p - a - ab * h);
}

fn wire(p: vec2f, a: vec2f, b: vec2f) -> f32 {
    return (1.0 - smoothstep(0.0, 1.2, segment_distance(p, a, b))) * 0.22;
}

// Nœud : disque sombre cerclé, halo proportionnel à son énergie (0..1).
fn node(p: vec2f, center: vec2f, color: vec3f, energy: f32) -> vec3f {
    let d = length(p - center);
    let ring = 1.0 - smoothstep(0.0, 1.5, abs(d - NODE_RADIUS));
    let core = 1.0 - smoothstep(NODE_RADIUS - 1.5, NODE_RADIUS, d);
    let halo = exp(-max(d - NODE_RADIUS, 0.0) / 12.0);
    return color * (core * (0.08 + 0.35 * energy) + ring * (0.5 + 0.5 * energy) + halo * 0.35 * energy);
}

fn ease(x: f32) -> f32 {
    return x * x * (3.0 - 2.0 * x);
}

// Kafka -> 3 jobs Flink indépendants -> Iceberg. Revenue et KPI : fenêtres qui se remplissent puis émettent ;
// AnomalyDetection (CEP) absorbe tout et n'émet qu'une alerte de temps en temps.
fn viz_flink(p: vec2f, size: vec2f, t: f32) -> vec3f {
    let source = vec2f(0.09, 0.5) * size;
    let sink = vec2f(0.88, 0.5) * size;
    var color = vec3f(0.0);
    var sink_energy = 0.0;
    for (var job = 0; job < 3; job++) {
        let center = vec2f(0.45, 0.2 + 0.3 * f32(job)) * size;
        color += ACCENT * (wire(p, source, center) + wire(p, center, sink));
        // Événements : le même flux arrive sur les trois jobs.
        for (var k = 0; k < 6; k++) {
            let phase = fract(t * 0.35 + f32(k) / 6.0);
            color += ACCENT_HOT * dot_glow(p, mix(source, center, phase), 3.0) * 0.9;
        }
        var energy: f32;
        var output_color: vec3f;
        var output_phase: f32;
        if (job == 1) {
            // Alerte CEP toutes les 4 s.
            output_phase = fract(t / 4.0 + 0.3) * 2.5;
            energy = 1.0 - smoothstep(0.0, 0.4, output_phase);
            output_color = RED;
        } else {
            // Fenêtre de 3 s (1 min simulée) : remplissage, puis émission d'un agrégat.
            let window = fract(t / 3.0 + f32(job) * 0.25);
            energy = window;
            output_phase = window * 3.0;
            output_color = GOLD;
        }
        if (output_phase < 1.0) {
            color += output_color * dot_glow(p, mix(center, sink, ease(output_phase)), 5.5) * 1.4;
            sink_energy = max(sink_energy, smoothstep(0.8, 1.0, output_phase));
        }
        color += node(p, center, select(ACCENT, RED, job == 1 && energy > 0.05), energy);
    }
    color += node(p, source, ACCENT, 0.6 + 0.4 * sin(t * 3.0));
    color += node(p, sink, GOLD, sink_energy);
    return color;
}

// P(majorité de n jurés juste), chacun juste avec la probabilité p : somme binomiale par récurrence des termes.
fn majority(n: i32, p: f32) -> f32 {
    var term = pow(1.0 - p, f32(n));
    var total = 0.0;
    for (var k = 0; k <= n; k++) {
        if (2 * k > n) {
            total += term;
        }
        term *= f32(n - k) / f32(k + 1) * p / (1.0 - p);
    }
    return total;
}

// Théorème du jury de Condorcet : courbes P(majorité) pour p = 0,6 / 0,5 / 0,4, tracées quand N grandit (1 à 51).
fn viz_condorcet(p: vec2f, size: vec2f, t: f32) -> vec3f {
    let uv = p / size;
    let left = 0.08;
    let right = 0.84;
    let top = 0.12;
    let bottom = 0.85;
    var color = vec3f(0.0);
    let pixel = 1.0 / size;

    // Axes : P = 1, P = 0,5 (pointillés), P = 0.
    let y_half = mix(bottom, top, 0.5);
    let in_x = step(left, uv.x) * step(uv.x, right);
    color += NEUTRAL * in_x * (1.0 - smoothstep(0.0, pixel.y * 1.2, abs(uv.y - top))) * 0.25;
    color += NEUTRAL * in_x * (1.0 - smoothstep(0.0, pixel.y * 1.2, abs(uv.y - bottom))) * 0.35;
    color += NEUTRAL * in_x * step(0.5, fract(uv.x * size.x / 8.0)) * (1.0 - smoothstep(0.0, pixel.y * 1.2, abs(uv.y - y_half))) * 0.3;

    // Tracé progressif sur 6 s, puis courbes complètes 2 s.
    let reveal = min(fract(t / 8.0) * 8.0 / 6.0, 1.0);
    let x = (uv.x - left) / (right - left);
    if (x < 0.0 || x > reveal) {
        return color;
    }
    // N impair entre 1 et 51, interpolé entre deux N successifs pour une courbe continue.
    let n_real = x * 25.0;
    let n_low = i32(floor(n_real));
    let blend = fract(n_real);
    let probabilities = array<f32, 3>(0.6, 0.5, 0.4);
    let colors = array<vec3f, 3>(GREEN, NEUTRAL, RED);
    for (var curve = 0; curve < 3; curve++) {
        let q = probabilities[curve];
        let value = mix(majority(2 * n_low + 1, q), majority(2 * n_low + 3, q), blend);
        let y = mix(bottom, top, value);
        let line = 1.0 - smoothstep(0.0, pixel.y * 2.0, abs(uv.y - y));
        let head = dot_glow(uv * size, vec2f(mix(left, right, reveal), y) * size, 5.0) * step(reveal - x, pixel.x * 6.0);
        color += colors[curve] * (line * 0.9 + head * 1.5);
    }
    return color;
}

// Position du jeton sur la jambe `leg` du pipeline AgenticDev (arc sous les nœuds pour la boucle de correction).
fn agents_leg(leg: i32, phase: f32, nodes: array<vec2f, 4>, height: f32) -> vec2f {
    let from_node = array<i32, 7>(0, 1, 2, 3, 2, 3, 2);
    let to_node = array<i32, 7>(1, 2, 3, 2, 3, 2, 3);
    let a = nodes[from_node[leg]];
    let b = nodes[to_node[leg]];
    let s = ease(phase);
    if (leg == 3 || leg == 5) {
        let control = (a + b) * 0.5 + vec2f(0.0, 0.45 * height);
        return mix(mix(a, control, s), mix(control, b, s), s);
    }
    return mix(a, b, s);
}

// Architect -> Designer -> Developer -> Tester ; deux échecs renvoient au Developer (fix loop), puis succès.
fn viz_agents(p: vec2f, size: vec2f, t: f32) -> vec3f {
    let nodes = array<vec2f, 4>(
        vec2f(0.12, 0.38) * size, vec2f(0.37, 0.38) * size, vec2f(0.62, 0.38) * size, vec2f(0.87, 0.38) * size,
    );
    let leg_duration = 1.3;
    let cycle = 7.0 * leg_duration + 2.4;
    let s = fract(t / cycle) * cycle;
    let leg = min(i32(s / leg_duration), 6);
    let phase = select(fract(s / leg_duration), 1.0, s >= 7.0 * leg_duration);

    var color = vec3f(0.0);
    for (var i = 0; i < 3; i++) {
        color += ACCENT * wire(p, nodes[i], nodes[i + 1]);
    }
    // Chemin de la boucle de correction : arc en pointillés sous Developer et Tester.
    var arc_distance = 1e9;
    for (var i = 0; i <= 16; i++) {
        arc_distance = min(arc_distance, length(p - agents_leg(3, f32(i) / 16.0, nodes, size.y)));
    }
    color += RED * (1.0 - smoothstep(0.0, 1.8, arc_distance)) * 0.35;

    // Verdict du Tester à l'arrivée des jambes 2 et 4 (échec) puis 6 (succès, maintenu jusqu'à la fin du cycle).
    var verdict = vec3f(0.0);
    var verdict_energy = 0.0;
    if (leg == 3 || leg == 5) {
        verdict = RED;
        verdict_energy = 1.0 - phase;
    } else if (s >= 7.0 * leg_duration) {
        verdict = GREEN;
        verdict_energy = 1.0;
    }
    for (var i = 0; i < 4; i++) {
        var node_color = ACCENT;
        var energy = 0.15;
        if (i == 3 && verdict_energy > 0.0) {
            node_color = verdict;
            energy = verdict_energy;
        }
        color += node(p, nodes[i], node_color, energy);
    }
    if (s < 7.0 * leg_duration) {
        let token_color = select(ACCENT_HOT, RED, leg == 3 || leg == 5);
        color += token_color * dot_glow(p, agents_leg(leg, phase, nodes, size.y), 6.0) * 1.6;
    }
    return color;
}

// Médaillon : bronze brut et désordonné, silver nettoyé et aligné, gold agrégé et stable.
fn viz_lakehouse(p: vec2f, size: vec2f, t: f32) -> vec3f {
    let uv = p / size;
    var color = vec3f(0.0);
    // Séparateurs entre les couches.
    for (var i = 1; i < 3; i++) {
        let x = f32(i) / 3.0;
        color += NEUTRAL * (1.0 - smoothstep(0.0, 1.0 / size.x, abs(uv.x - x))) * 0.2 * step(0.12, uv.y) * step(uv.y, 0.78);
    }
    // Bronze : 36 points qui s'agitent, certains s'éteignent (données manquantes) ou se dédoublent.
    for (var i = 0; i < 36; i++) {
        let seed = f32(i);
        let base = vec2f(0.04 + 0.26 * hash(seed), 0.15 + 0.6 * hash(seed + 13.0));
        let wobble = vec2f(sin(t * (1.3 + hash(seed + 3.0) * 2.0) + seed), cos(t * (1.1 + hash(seed + 7.0) * 2.0) + seed)) * 0.025;
        let flicker = select(1.0, 0.5 + 0.5 * sin(t * 6.0 + seed), hash(seed + 21.0) < 0.25);
        color += BRONZE * dot_glow(p, (base + wobble) * size, 2.6) * flicker;
    }
    // Passage bronze -> silver : flux régulier sur 4 couloirs.
    for (var i = 0; i < 8; i++) {
        let lane = f32(i % 4);
        let phase = fract(t * 0.4 + f32(i) / 8.0);
        let position = vec2f(mix(0.30, 0.36, phase), 0.2 + 0.15 * lane);
        color += SILVER * dot_glow(p, position * size, 2.2) * 0.6;
    }
    // Silver : points alignés sur 4 couloirs, qui avancent régulièrement.
    for (var row = 0; row < 4; row++) {
        for (var column = 0; column < 5; column++) {
            let x = 0.37 + 0.26 * fract((f32(column) + t * 0.4) / 5.0);
            color += SILVER * dot_glow(p, vec2f(x, 0.2 + 0.15 * f32(row)) * size, 2.4);
        }
    }
    // Passage silver -> gold : quelques paquets plus gros (agrégation).
    for (var i = 0; i < 3; i++) {
        let phase = fract(t * 0.25 + f32(i) / 3.0);
        color += GOLD * dot_glow(p, vec2f(mix(0.63, 0.70, phase), 0.3 + 0.2 * f32(i)) * size, 3.5) * 0.7;
    }
    // Gold : 6 agrégats stables qui respirent, reliés entre eux.
    for (var i = 0; i < 6; i++) {
        let center = vec2f(0.76 + 0.12 * f32(i % 2), 0.25 + 0.2 * f32(i / 2)) * size;
        let breath = 0.75 + 0.25 * sin(t * 1.5 + f32(i));
        color += GOLD * dot_glow(p, center, 6.5) * breath * 1.2;
        if (i + 2 < 6) {
            color += GOLD * wire(p, center, vec2f(0.76 + 0.12 * f32(i % 2), 0.25 + 0.2 * f32(i / 2 + 1)) * size) * 0.8;
        }
    }
    return color;
}

fn viz(variant: u32, p: vec2f, size: vec2f, t: f32) -> vec3f {
    switch variant {
        case VIZ_FLINK: { return viz_flink(p, size, t); }
        case VIZ_CONDORCET: { return viz_condorcet(p, size, t); }
        case VIZ_AGENTS: { return viz_agents(p, size, t); }
        case VIZ_LAKEHOUSE: { return viz_lakehouse(p, size, t); }
        default: { return vec3f(0.0); }
    }
}
