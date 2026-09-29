// Page 404 : le curseur capture les particules qu'il frôle, qui rejoignent alors leur place dans le « 404 ».

@group(0) @binding(2) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(3) var<storage, read_write> captured_count: atomic<u32>;

const CAPTURE_RADIUS: f32 = 70.0;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3u) {
    let index = id.x;
    if (index >= arrayLength(&particles)) {
        return;
    }
    var target_rect = Rect();
    var found = false;
    for (var i = 0u; i < globals.rect_count; i++) {
        if (rects[i].kind == KIND_TARGET) {
            target_rect = rects[i];
            found = true;
        }
    }
    if (!found) {
        return;
    }

    var particle = particles[index];
    let dt = globals.dt;

    if (particle.captured > 0.5) {
        // Ressort amorti vers la place dans le « 404 » ; la cible suit l'ancre au scroll.
        let goal = target_rect.min + particle.slot * (target_rect.max - target_rect.min);
        particle.vel = mix(particle.vel, (goal - particle.pos) * 5.0, min(dt * 4.0, 1.0));
        particle.pos += particle.vel * dt;
    } else {
        particle.pos.y -= globals.scroll_delta * 0.35;
        var force = flow(particle.pos, globals.time) * 22.0;
        let to_pointer = globals.pointer - particle.pos;
        let pointer_distance = length(to_pointer);
        if (pointer_distance < CAPTURE_RADIUS) {
            particle.captured = 1.0;
            atomicAdd(&captured_count, 1u);
        } else if (pointer_distance < 260.0) {
            force += to_pointer / pointer_distance * (1.0 - pointer_distance / 260.0) * 120.0;
        }
        particle.vel = (particle.vel + force * dt) * pow(0.12, dt);
        particle.pos = wrap(particle.pos + particle.vel * dt);
    }
    particles[index] = particle;
}
