struct Camera {
    mat_projection: mat4x4<f32>,
    mat_view: mat4x4<f32>,
}
@group(0) @binding(0) var<uniform> u_camera: Camera;

struct BoxData {
    model_matrix: mat4x4<f32>,
    min: vec3<f32>,
    _p1: f32,
    max: vec3<f32>,
    _p2: f32,
}
@group(1) @binding(0) var<uniform> u_box_data: BoxData;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
}

@vertex
fn vertex_main(vertex: VertexInput) -> VertexOutput {
    let size = u_box_data.max - u_box_data.min;
    let center = u_box_data.min + size / 2.0;

    let mvp = u_camera.mat_projection * u_camera.mat_view * u_box_data.model_matrix;

    return VertexOutput(
        mvp * vec4((vertex.position * size) + center, 1.0),
        vertex.normal,
        vertex.tex_coord.xy,
    );
}

@fragment
fn fragment_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(0.5, 1.0, 0.5, 0.2);
}

@fragment
fn fragment_main_wireframe(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(0.0, 0.5, 0.0, 1.0);
}
