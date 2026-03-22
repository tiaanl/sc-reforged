struct Viewport {
    size: vec2<f32>,
}

struct Vertex {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> viewport: Viewport;

@vertex
fn vertex(vertex: Vertex) -> VertexOut {
    let ndc = vec2<f32>(
        (vertex.pos.x / viewport.size.x) * 2.0 - 1.0,
        1.0 - (vertex.pos.y / viewport.size.y) * 2.0,
    );

    return VertexOut(
        vec4<f32>(ndc, 0.0, 1.0),
        vertex.color,
    );
}

@fragment
fn fragment(vertex: VertexOut) -> @location(0) vec4<f32> {
    return vertex.color;
}
