#define_import_path shadows

const MAX_CASCADES: u32 = 4;  // Must sync with ShadowCascades::MAX_CASCADES.

struct Cascades {
    cascades: array<mat4x4<f32>, MAX_CASCADES>,
    count: u32,
}

fn project_to_light_ndc(light_view_proj: mat4x4<f32>, world_position: vec3<f32>) -> vec3<f32> {
    let light_clip_position = light_view_proj * vec4<f32>(world_position, 1.0);

    // If behind the "light camera", "exit" out with an invalid value.
    if light_clip_position.w <= 0.0 {
        return vec3<f32>(2.0, 2.0, 2.0);
    }

    return light_clip_position.xyz / light_clip_position.w;
}

fn sample_shadow_pcf_3x3(
    shadow_maps: texture_depth_2d_array,
    sampler_cmp: sampler_comparison,
    index: u32,
    tex_coord: vec2<f32>,
    depth_ref: f32,
) -> f32 {
    let size = vec2<f32>(textureDimensions(shadow_maps, 0));
    let texel = 1.0 / size;

    // Offsets for a 3x3 kernel.
    const OFFSETS = array<vec2<f32>, 9>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(0.0, -1.0), vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, 0.0), vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0),
        vec2<f32>(-1.0, 1.0), vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 1.0),
    );

    var sum: f32 = 0.0;
    for (var i = 0u; i < 9u; i += 1u) {
        sum += textureSampleCompare(
            shadow_maps,
            sampler_cmp,
            tex_coord + OFFSETS[i] * texel,
            index,
            depth_ref,
        );
    }

    return sum / 9.0;
}

// Simple slope-scaled depth bias in "depth" space.
fn depth_bias(world_normal: vec3<f32>, world_light_dir: vec3<f32>) -> f32 {
    let N = normalize(world_normal);
    let L = normalize(-world_light_dir);  // Light to surface.

    let slope = 1.0 - clamp(dot(N, L), 0.0, 1.0);  // 0 when facing the light, 1 when grazing.

    // TODO: Tune
    let const_bias = 0.0008; // In depth units.
    let slope_bias = 0.0025; // Scales with grazing angle.

    return const_bias + slope_bias * slope;
}
