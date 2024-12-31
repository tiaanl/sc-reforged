#import world::camera
#import world::fog

@group(0) @binding(0) var t_terrain_texture: texture_2d<f32>;
@group(0) @binding(1) var s_terrain_texture: sampler;

@group(1) @binding(0) var<uniform> u_camera: camera::Camera;
@group(2) @binding(0) var<uniform> u_fog: fog::Fog;

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

@fragment
fn fragment_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(t_terrain_texture, s_terrain_texture, vertex.tex_coord);
    let fog_factor = fog::fog_factor(u_fog, vertex.world_position, u_camera.position.xyz);
    let final_color = mix(tex_color, vec4(u_fog.color, 1.0), fog_factor);
    return final_color;
}
