struct Matrices {
    projection: mat4x4<f32>,
    view: mat4x4<f32>,
}
@group(0) @binding(0) var<uniform> u_matrices: Matrices;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) world_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vertex_main(vertex: VertexInput) -> VertexOutput {
    return VertexOutput(
        u_matrices.projection * u_matrices.view * vertex.position,
        vertex.color
    );
}

@fragment
fn fragment_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vertex.color;
}