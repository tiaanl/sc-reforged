#define_import_path world::geometry_buffers

struct GeometryBuffers {
    @location(0) albedo: vec4<f32>,
    @location(1) position: vec4<f32>,
    @location(2) normal: vec4<f32>,
    @location(3) alpha_accumulation: vec4<f32>,
    @location(4) alpha_revealabe: f32,
    @location(5) entity_id: u32,
}
