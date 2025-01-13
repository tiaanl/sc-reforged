#import world::camera

@group(0) @binding(0) var t_terrain_texture: texture_2d<f32>;
@group(0) @binding(1) var s_terrain_texture: sampler;

@group(1) @binding(0) var<uniform> u_camera: camera::Camera;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
}

struct InstanceInput {
    @location(3) model0: vec4<f32>,
    @location(4) model1: vec4<f32>,
    @location(5) model2: vec4<f32>,
    @location(6) model3: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) world_position: vec3<f32>,
}

@vertex
fn vertex_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model = mat4x4<f32>(instance.model0, instance.model1, instance.model2, instance.model3);

    let world_position = (model * vec4(vertex.position, 1.0)).xyz;
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
    let color = textureSample(t_terrain_texture, s_terrain_texture, vertex.tex_coord);

    return color;
}

@fragment
fn ck_fragment_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_terrain_texture, s_terrain_texture, vertex.tex_coord);
    if color.x == 0.0 && color.y == 0.0 && color.z == 0 {
        discard;
    }

    return color;
}
