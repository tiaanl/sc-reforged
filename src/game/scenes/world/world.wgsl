struct Camera {
    projection: mat4x4<f32>,
    view: mat4x4<f32>,
}
@group(0) @binding(0) var<uniform> u_camera: Camera;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) camera_direction: vec3<f32>,
}

@vertex
fn vertex_main(vertex: VertexInput) -> VertexOutput {
    let view_forward = vec4(0.0, 0.0, -1.0, 0.0);
    let inverse = transpose(u_camera.view); // No scaling, so just transpose.
    let camera_direction = normalize(inverse * view_forward);

    return VertexOutput(
        u_camera.projection * u_camera.view * vec4(vertex.position, 1.0),
        camera_direction.xyz,
    );
}

@fragment
fn fragment_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(1.0, 0.0, 0.0, 1.0);
}
