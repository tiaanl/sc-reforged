#define_import_path world::geometry_buffers

struct OpaqueGeometryBuffers {
    @location(0) albedo: vec4<f32>,
    @location(1) position_id: vec4<f32>,
}

fn to_opaque_geometry_buffer(albedo: vec3<f32>, position: vec3<f32>, entity_id: u32) -> OpaqueGeometryBuffers {
    return OpaqueGeometryBuffers(
        vec4<f32>(albedo, 1.0),
        vec4<f32>(position, bitcast<f32>(entity_id)),
    );
}

struct AlphaGeometryBuffers {
    @location(0) oit_accumulation: vec4<f32>,
    @location(1) oit_revealage: vec4<f32>,
}

fn to_alpha_geometry_buffer(base_color: vec3<f32>, alpha: f32, weight: f32) -> AlphaGeometryBuffers {
    let a = clamp(alpha, 0.0, 1.0);
    let w = max(weight, 0.0);

    // Accumulation: premultiplied color + accumulated weight
    let premultiplied = base_color * a * w;
    let accumulation  = vec4<f32>(premultiplied, a * w);

    // Revealage: only alpha channel is meaningful (source α drives (1-α) multiplicative blend)
    let revealage = vec4<f32>(0.0, 0.0, 0.0, a);

    return AlphaGeometryBuffers(accumulation, revealage);
}
