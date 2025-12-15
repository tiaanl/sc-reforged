struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@group(0) @binding(0)
var u_color: texture_2d<f32>;

@group(0) @binding(1)
var u_sampler: sampler;

// Meant to be called with 3 vertex indices: 0, 1, 2. Draws one large triangle
// over the clip space like this:
//
// -1,1         1,1
// ----------------------------
// |            *           .
// |            *         .
// |            *       .
// |            *     .
// |            *   .
// |            * .
// |*************
// |          . 1,-1
// |        .
// |      .
// |    .
// |  .
// |.
@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let x = i32(vertex_index) / 2;
    let y = i32(vertex_index) & 1;

    let tex_coords = vec2<f32>(f32(x) * 2.0, f32(y) * 2.0);

    let position = vec4<f32>(
        tex_coords.x * 2.0 - 1.0,
        1.0 - tex_coords.y * 2.0,
        0.0, 1.0
    );

    return VertexOutput(position, tex_coords);
}

@fragment
fn fragment(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(u_color, u_sampler, vertex.tex_coords);
}
