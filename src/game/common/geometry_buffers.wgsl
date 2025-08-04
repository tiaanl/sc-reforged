#define_import_path world::geometry_buffers

struct GeometryBuffers {
    @location(0) albedo: vec4<f32>,
    @location(1) position_id: vec4<f32>,
}

fn to_geometry_buffer(albedo: vec4<f32>, position: vec3<f32>, entity_id: u32) -> GeometryBuffers {
    return GeometryBuffers(
        albedo,
        vec4<f32>(position, bitcast<f32>(entity_id)),
    );
}
