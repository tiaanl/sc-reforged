@group(0) @binding(0) var t_screen: texture_2d<f32>;
@group(0) @binding(1) var s_screen: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
}

@vertex
fn vertex_main(
    @builtin(vertex_index) vertex_index: u32
) -> VertexOutput {
    let tex_coord = vec2<f32>(
        f32(vertex_index >> 1u),
        f32(vertex_index & 1u)
    ) * 2.0;
    let position = vec4<f32>(
        tex_coord * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0),
        0.0,
        1.0
    );

    return VertexOutput(position, tex_coord);
}

@fragment
fn fragment_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_screen, s_screen, vertex.tex_coord);
    return color;
}
