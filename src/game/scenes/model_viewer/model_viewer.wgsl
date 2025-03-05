#import world::camera

@group(0) @binding(0) var t_texture: texture_2d<f32>;
@group(0) @binding(1) var s_sampler: sampler;

@group(1) @binding(0) var<uniform> u_camera: camera::Camera;

struct NodeData {
    transform: mat4x4<f32>,
    parent: u32,
}

@group(2) @binding(0) var<storage, read> u_node_data: array<NodeData>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
    @location(3) node_index: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
}

@vertex fn vertex(vertex: VertexInput) -> VertexOutput {
    let mat_model = u_node_data[vertex.node_index].transform;

    let world_position = (mat_model * vec4<f32>(vertex.position, 1.0)).xyz;

    let clip_position = u_camera.mat_projection * u_camera.mat_view * vec4<f32>(world_position, 1.0);

    return VertexOutput(
        clip_position,
        vertex.tex_coord,
    );
}

@fragment fn fragment(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let base_color = textureSample(t_texture, s_sampler, vertex.tex_coord);

    return base_color;
}
