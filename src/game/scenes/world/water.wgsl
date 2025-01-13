#import world::camera

@group(0) @binding(0) var t_water: texture_2d<f32>;
@group(0) @binding(1) var s_water: sampler;

@group(1) @binding(0) var<uniform> u_camera: camera::Camera;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) world_position: vec3<f32>,
}

@vertex
fn vertex_main(vertex: VertexInput) -> VertexOutput {
    let world_position = vertex.position;
    let proj_view = u_camera.mat_projection * u_camera.mat_view;
    let clip_position = proj_view * vec4(world_position, 1.0);

    return VertexOutput(
        clip_position,
        vertex.normal,
        vertex.tex_coord,
        world_position,
    );
}

@fragment
fn fragment_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_water, s_water, vertex.tex_coord);

    return color;
}
