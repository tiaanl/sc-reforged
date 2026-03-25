struct Viewport {
    size: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> viewport: Viewport;

@group(1) @binding(0)
var t: texture_2d<f32>;
@group(1) @binding(1)
var s: sampler;

struct RectParams {
    pos: vec2<f32>,
    size: vec2<f32>,
    alpha: f32,
}

var<push_constant> rect: RectParams;

struct Vertex {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vertex(vertex: Vertex) -> VertexOut {
    let pixel_pos = rect.pos + vertex.pos * rect.size;

    let ndc = vec2<f32>(
        (pixel_pos.x / viewport.size.x) * 2.0 - 1.0,
        1.0 - (pixel_pos.y / viewport.size.y) * 2.0,
    );

    return VertexOut(
        vec4<f32>(ndc, 0.0, 1.0),
        vertex.uv,
        vertex.color,
    );
}

@fragment
fn fragment(vertex: VertexOut) -> @location(0) vec4<f32> {
    let base = textureSample(t, s, vertex.uv);
    let out = base * vertex.color;
    return vec4<f32>(out.rgb, out.a * rect.alpha);
}
