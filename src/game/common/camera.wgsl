#define_import_path world::camera

struct Camera {
    mat_proj_view: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
    frustum: array<vec4<f32>, 6>,
}

fn camera_forward(camera: Camera) -> vec3<f32> {
    let normal = camera.frustum[4].xyz;
    let length = max(length(normal), 1e-6);
    return normal / length;
}
