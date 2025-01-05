#import world::camera

@group(0) @binding(0) var t_terrain_texture: texture_2d<f32>;
@group(0) @binding(1) var s_terrain_texture: sampler;

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
    let clip_position = u_camera.mat_projection * u_camera.mat_view * vec4(world_position, 1.0);

    return VertexOutput(
        clip_position,
        vertex.normal,
        vertex.tex_coord,
        world_position,
    );
}

struct FragmentOutput {
    @location(0) albedo: vec4<f32>,
    @location(1) position: vec4<f32>,
}

@fragment
fn fragment_main(vertex: VertexOutput) -> FragmentOutput {
    let tex_color = textureSample(t_terrain_texture, s_terrain_texture, vertex.tex_coord);

    let distance_to_camera = length(u_camera.position - vertex.world_position);

    return FragmentOutput(
        tex_color,
        vec4<f32>(vertex.world_position, distance_to_camera),
    );
}

@fragment
fn fragment_main_wireframe(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(1.0, 1.0, 0.0, 1.0);
}
