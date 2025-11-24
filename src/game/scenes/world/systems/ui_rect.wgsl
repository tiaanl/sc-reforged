struct UiState {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> ui_state: UiState;

struct InstanceInput {
    @location(0) min: vec2<f32>,
    @location(1) max: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

const CORNERS = array<vec2<f32>, 4>(
    vec2<f32>(0.0, 0.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(0.0, 1.0),
    vec2<f32>(1.0, 1.0),
);

@vertex
fn vertex(
    instance: InstanceInput,
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    let corner = CORNERS[vertex_index];
    let v = instance.min + (instance.max - instance.min) * corner;

    let clip_position = ui_state.view_proj * vec4<f32>(v, 0.0, 1.0);

    return VertexOutput(clip_position, instance.color);
}

@fragment
fn fragment(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vertex.color;
}
