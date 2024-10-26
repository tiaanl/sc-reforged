struct Camera {
    mat_projection: mat4x4<f32>,
    mat_view: mat4x4<f32>,
}
@group(0) @binding(0) var<uniform> u_camera: Camera;

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
    return VertexOutput(
        u_camera.mat_projection * u_camera.mat_view * vec4(vertex.position, 1.0),
        vertex.normal,
        vertex.tex_coord,
    );
}

@vertex
fn vertex_main_wireframe(vertex: VertexInput) -> VertexOutput {
    return VertexOutput(
        u_camera.mat_projection * u_camera.mat_view * (vec4(vertex.position, 1.0) + vec4(0.0, 0.0, 0.0, 0.0)),
        vertex.normal,
        vertex.tex_coord,
    );
}

@fragment
fn fragment_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let sun_dir = normalize(vec3(1.0, 1.0, 0.0));
    let c = dot(sun_dir, vertex.normal);

    return vec4(c, c, c, 1.0);
}

@fragment
fn fragment_main_wireframe(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(1.0, 1.0, 0.0, 1.0);
}
