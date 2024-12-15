#define_import_path world::fog

struct Fog {
    color: vec3<f32>,
    start: f32,
    end: f32,
}

fn fog_factor(fog: Fog, position: vec3<f32>, camera_position: vec3<f32>) -> f32 {
    let distance = length(position - camera_position);
    let fog_factor = clamp((distance - fog.start) / (fog.end - fog.start), 0.0, 1.0);
    return fog_factor;
}
