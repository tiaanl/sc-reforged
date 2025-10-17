#define_import_path geometry_buffer

struct OpaqueGeometryBuffer {
    @location(0) color: vec4<f32>,
}

fn to_opaque_geometry_buffer(color: vec3<f32>) -> OpaqueGeometryBuffer {
    return OpaqueGeometryBuffer(vec4<f32>(color, 1.0));
}

struct AlphaGeometryBuffer {
    @location(0) oit_accumulation: vec4<f32>,
    @location(1) oit_revealage: vec4<f32>,
}

fn to_alpha_geometry_buffer(base_color: vec3<f32>, alpha: f32, weight: f32) -> AlphaGeometryBuffer {
    let a = clamp(alpha, 0.0, 1.0);
    let w = max(weight, 0.0);

    // Accumulation: premultiplied color + accumulated weight
    let premultiplied = base_color * a * w;
    let accumulation  = vec4<f32>(premultiplied, a * w);

    // Revealage: only alpha channel is meaningful (source α drives (1-α) multiplicative blend)
    let revealage = vec4<f32>(0.0, 0.0, 0.0, a);

    return AlphaGeometryBuffer(accumulation, revealage);
}
