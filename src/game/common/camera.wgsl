#define_import_path world::camera

struct Camera {
    mat_projection: mat4x4<f32>,
    mat_view: mat4x4<f32>,
}
