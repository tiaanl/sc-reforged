#define_import_path camera_env

struct CameraEnv {
    proj_view: mat4x4<f32>,
    frustum: array<vec4<f32>, 6>,
    position: vec4<f32>,
    forward: vec4<f32>,

    sun_dir: vec4<f32>,       // x, y, z, 0
    sun_color: vec4<f32>,     // r, g, b, 1
    ambient_color: vec4<f32>, // r, g, b, 1
    fog_color: vec4<f32>,     // r, g, b, 1
    fog_distance: f32,
    fog_near_fraction: f32,
}

/// Diffuse + ambient lighting, modulated by shadow visibility.
fn diffuse(
    env: CameraEnv,
    normal: vec3<f32>,
    base_color: vec3<f32>,
    visibility: f32,              // 0 = full shadow, 1 = fully lit
) -> vec3<f32> {
    let N = normalize(normal);
    let L = -normalize(env.sun_dir.xyz); // from fragment toward sun

    let n_dot_l = max(dot(N, L), 0.0);

    // Direct sunlight (scaled by visibility)
    let sun_light = env.sun_color.rgb * n_dot_l * visibility;
    let ambient = env.ambient_color.rgb;

    return (sun_light + ambient) * base_color;
}

/// Same as diffuse_with_fog(), but blends in a shadow term.
fn diffuse_with_fog(
    env: CameraEnv,
    normal: vec3<f32>,
    base_color: vec3<f32>,
    distance: f32,
    visibility: f32,
) -> vec3<f32> {
    let lit_color = diffuse(env, normal, base_color, visibility);

    let fog_near = env.fog_distance * env.fog_near_fraction;
    let fog_far = env.fog_distance;

    let fog_factor = linear_fog_factor(fog_near, fog_far, distance);

    return mix(lit_color, env.fog_color.rgb, fog_factor);
}

fn linear_fog_factor(fog_near: f32, fog_far: f32, distance: f32) -> f32 {
    return clamp((distance - fog_near) / (fog_far - fog_near), 0.0, 1.0);
}

