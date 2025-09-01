#define_import_path environment

struct Environment {
    sun_dir: vec4<f32>,    // x, y, z, ambient.r
    sun_color: vec4<f32>,  // r, g, b, ambient.g
    fog_color: vec4<f32>,  // r, g, b, ambient.b
    fog_params: vec4<f32>, // near, far, PADDING, PADDING
    sun_proj_view: mat4x4<f32>,
}

fn linear_fog_factor(fog_near: f32, fog_far: f32, distance: f32) -> f32 {
    return clamp((distance - fog_near) / (fog_far - fog_near), 0.0, 1.0);
}

/// Diffuse + ambient lighting, modulated by shadow visibility.
fn diffuse(
    env: Environment,
    normal: vec3<f32>,
    base_color: vec3<f32>,
    visibility: f32,              // 0 = full shadow, 1 = fully lit
) -> vec3<f32> {
    let N = normalize(normal);
    let L = -normalize(env.sun_dir.xyz); // from fragment toward sun

    let n_dot_l = max(dot(N, L), 0.0);

    // Direct sunlight (scaled by visibility)
    let sun_light = env.sun_color.rgb * n_dot_l * visibility;

    // Ambient term (not shadowed)
    let ambient = vec3<f32>(env.sun_dir.w, env.sun_color.w, env.fog_color.w);
    let ambient_color = env.sun_color.rgb * ambient;

    let lighting = sun_light + ambient_color;

    return lighting * base_color;
}

/// Same as diffuse_with_fog(), but blends in a shadow term.
fn diffuse_with_fog_shadow(
    env: Environment,
    normal: vec3<f32>,
    base_color: vec3<f32>,
    distance: f32,
    visibility: f32,
) -> vec3<f32> {
    let lit_color = diffuse(env, normal, base_color, visibility);

    let fog_factor = linear_fog_factor(
        env.fog_params.x, // near
        env.fog_params.y, // far
        distance,
    );

    return mix(lit_color, env.fog_color.rgb, fog_factor);
}

/// Same as diffuse(), but blends fog on top.
fn diffuse_with_fog(
    env: Environment,
    normal: vec3<f32>,
    base_color: vec3<f32>,
    distance: f32,
) -> vec3<f32> {
    return diffuse_with_fog_shadow(env, normal, base_color, distance, 1.0);
}

