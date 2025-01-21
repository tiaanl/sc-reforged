#define_import_path world::environment

struct Environment {
    sun_dir: vec4<f32>,    // x, y, z, PADDING
    sun_color: vec4<f32>,  // r, g, b, PADDING
    fog_color: vec4<f32>,  // r, g, b, PADDING
    fog_params: vec4<f32>, // near, far, PADDING, PADDING
}

fn linear_fog_factor(fog_near: f32, fog_far: f32, distance: f32) -> f32 {
    return clamp((distance - fog_near) / (fog_far - fog_near), 0.0, 1.0);
}

fn diffuse(
    env: Environment,
    normal: vec3<f32>,
    base_color: vec3<f32>,
) -> vec3<f32> {
    let N = normalize(normal);

    // Direction from the terrain to the sun.
    let L = -normalize(env.sun_dir.xyz);

    // Amount of diffuse light emitted from the surface.
    let diffuse_factor = max(dot(N, L), 0.0);

    // The lit base color.
    let result = diffuse_factor * base_color;

    return result;
}

fn diffuse_with_fog(
    env: Environment,
    normal: vec3<f32>,
    base_color: vec3<f32>,
    distance: f32,
) -> vec3<f32> {
    let diffuse = diffuse(env, normal, base_color);

    let fog_factor = linear_fog_factor(
        env.fog_params.x, // near
        env.fog_params.y, // far
        distance,
    );

    let result = mix(diffuse, env.fog_color.rgb, fog_factor);

    return result;
}
