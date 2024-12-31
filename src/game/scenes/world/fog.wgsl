#define_import_path world::fog

struct Fog {
    color: vec3<f32>,
    _padding: f32,
    start: f32,
    end: f32,
    density: f32,
}

fn normalized_distance(fog: Fog, distance: f32) -> f32 {
    return clamp((distance - fog.start) / (fog.end - fog.start), 0.0, 1.0);
}

fn linear_fog_factor(fog: Fog, position: vec3<f32>, camera_position: vec3<f32>) -> f32 {
    let distance = length(position - camera_position);
    return normalized_distance(fog, distance);
}

fn exp_fog_factor(fog: Fog, position: vec3<f32>, camera_position: vec3<f32>) -> f32 {
    let distance = length(position - camera_position);
    let normalized_distance = normalized_distance(fog, distance);
    let fog_factor = exp(-pow(normalized_distance * fog.density, 2.0));
    return 1.0 - fog_factor;
}

fn fog_factor(fog: Fog, position: vec3<f32>, camera_position: vec3<f32>) -> f32 {
    return linear_fog_factor(fog, position, camera_position);
}
