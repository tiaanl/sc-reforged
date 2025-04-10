#import world::camera
#import world::geometry_buffers

@group(0) @binding(0) var<uniform> u_camera: camera::Camera;

@group(1) @binding(0) var t_texture: texture_2d<f32>;
@group(1) @binding(1) var s_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
    @location(3) node_index: u32,
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

    // We don't scale objects, so the model matrix without translation is good for now.
    // let world_normal = (model * vec4<f32>(vertex.normal, 0.0)).xyz;
    let world_normal = vertex.normal;

    return VertexOutput(
        clip_position,
        world_normal,
        vertex.tex_coord,
        world_position,
    );
}

@fragment
fn fragment_main(vertex: VertexOutput) -> geometry_buffers::GeometryBuffers {
    let base_color = textureSample(t_texture, s_sampler, vertex.tex_coord);

    return geometry_buffers::GeometryBuffers(
        base_color,
        vec4<f32>(vertex.world_position, 1.0),
        vec4<f32>(vertex.normal, 1.0),
        0,
    );
}
