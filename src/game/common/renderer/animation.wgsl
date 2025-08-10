#define_import_path world::animation

const FPS: f32 = 30.0;
const LOOPING: bool = true;

struct FrameLerp {
    before: u32,
    after: u32,
    t: f32,
};

fn compute_frame_lerp(global_time: f32, frame_count: u32) -> FrameLerp {
    let clamped_count = max(frame_count, 1u);
    let last_frame = clamped_count - 1u;

    var fractional_frame = global_time * FPS;
    if (LOOPING && clamped_count > 1u) {
        // fractional_frame mod frame_count
        fractional_frame =
            fractional_frame - floor(fractional_frame / f32(clamped_count)) * f32(clamped_count);
    } else {
        fractional_frame = clamp(fractional_frame, 0.0, f32(last_frame));
    }

    let before_frame = u32(floor(fractional_frame));
    var after_frame: u32;
    if (LOOPING && clamped_count > 1u) {
        after_frame = (before_frame + 1u) % clamped_count;
    } else {
        after_frame = min(before_frame + 1u, last_frame);
    }

    let denom = max(1.0, f32(after_frame) - f32(before_frame));
    let t = clamp((fractional_frame - f32(before_frame)) / denom, 0.0, 1.0);

    return FrameLerp(before_frame, after_frame, t);
}

struct PositionAndRotation {
    position: vec4<f32>,
    rotation: vec4<f32>,
};

fn load_position_and_rotation(
    positions: texture_2d<f32>,
    rotations: texture_2d<f32>,
    frame: u32,
    node_index: u32,
) -> PositionAndRotation {
    let position = textureLoad(positions, vec2<i32>(i32(frame), i32(node_index)), 0);
    let rotation = textureLoad(rotations, vec2<i32>(i32(frame), i32(node_index)), 0);
    return PositionAndRotation(position, rotation);
}

fn local_transform_for_time(
    positions: texture_2d<f32>,
    rotations: texture_2d<f32>,
    node_index: u32,
    time: f32,
) -> mat4x4<f32> {
    let frame_count = max(textureDimensions(positions, 0).x, 1u);

    let lerp = compute_frame_lerp(time, frame_count);

    let before = load_position_and_rotation(positions, rotations, lerp.before, node_index);
    let after  = load_position_and_rotation(positions, rotations, lerp.after,  node_index);

    let interpolated_position = mix(before.position, after.position, lerp.t).xyz;

    // Normalize the quaternion after LERP (nlerp) to keep it unit-length.
    let q = mix(before.rotation, after.rotation, lerp.t);
    let interpolated_rotation = q / max(length(q), math::EPSILON);

    return math::transform_from_position_and_rotation(
        interpolated_position,
        interpolated_rotation,
    );
}
