#import camera_env::CameraEnv;
#import geometry_buffer::{AlphaGeometryBuffer, to_alpha_geometry_buffer};

@group(0) @binding(0)
var<uniform> u_camera_env: CameraEnv;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
}

struct InstanceInput {
    @location(2) transform_0: vec4<f32>,
    @location(3) transform_1: vec4<f32>,
    @location(4) transform_2: vec4<f32>,
    @location(5) transform_3: vec4<f32>,
    @location(6) half_extent: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
}

@vertex
fn vertex(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    let transform = mat4x4<f32>(
        instance.transform_0,
        instance.transform_1,
        instance.transform_2,
        instance.transform_3,
    );

    let world_position = transform * (vec4<f32>(vertex.position * instance.half_extent, 1.0));

    let clip_position = u_camera_env.proj_view * world_position;

    return VertexOutput(clip_position, vertex.tex_coord);
}

@fragment
fn fragment(vertex: VertexOutput) -> AlphaGeometryBuffer {
    let color = vec3<f32>(0.0, 1.0, 0.0);

    // Distance to the nearest edge of the quad.
    // 0.0 at the edges and 0.5 at the center.
    let distance = min(
        min(vertex.tex_coord.x, 1.0 - vertex.tex_coord.x),
        min(vertex.tex_coord.y, 1.0 - vertex.tex_coord.y),
    );

    let feather = 0.1;
    let edge_factor = 1.0 - smoothstep(0.0, feather, distance);

    let edge_pow = 1.5;
    let alpha = pow(edge_factor, edge_pow);

    return to_alpha_geometry_buffer(color, alpha, 1.0);
}
