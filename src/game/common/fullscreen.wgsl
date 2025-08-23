#define_import_path fullscreen

/// Given a vertex index, returns the clip position for each point of a triangle
/// covering the entire screen.
fn clip_position(vertex_index: u32) -> vec4<f32> {
    let uv = vec2<f32>(f32(vertex_index >> 1u), f32(vertex_index & 1u)) * 2.0;
    let clip_position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);

    return clip_position;
}

/// Calculate a texture coordinate to sample a pixel from a fullscreen buffer.
/// (0..width - 1, 0..height - 1).
fn tex_coord(
    texture_dimensions: vec2<u32>,
    vertex_position: vec2<f32>,
) -> vec2<i32> {
    let tex_coord = vec2<i32>(clamp(
        vec2<i32>(vertex_position),
        vec2<i32>(0, 0),
        vec2<i32>(i32(texture_dimensions.x) - 1, i32(texture_dimensions.y) - 1),
    ));

    return tex_coord;
}
