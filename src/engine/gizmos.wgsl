#import camera_env::CameraEnv;

@group(0) @binding(0)
var<uniform> u_camera: CameraEnv;

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
        u_camera.proj_view * vertex.position,
        vertex.color
    );
}

@fragment
fn fragment_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vertex.color;
}