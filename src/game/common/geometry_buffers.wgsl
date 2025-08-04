#define_import_path world::geometry_buffers

struct OpaqueGeometryBuffers {
    @location(0) albedo: vec4<f32>,
    @location(1) position: vec4<f32>,
    @location(2) entity_id: u32,
}

struct AlphaGeometryBuffers {
    @location(0) entity_id: u32,
}
