#import world::camera
#import world::geometry_buffers

@group(0) @binding(0) var<uniform> u_camera: camera::Camera;

@group(1) @binding(0) var t_texture: texture_2d<f32>;
@group(1) @binding(1) var s_sampler: sampler;

struct Node {
    parent: u32,
    _d0: u32,
    _d2: u32,
    _d3: u32,
    transform: mat4x4<f32>,
}

@group(2) @binding(0) var<storage> u_nodes: array<Node>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
    @location(3) node_index: u32,
}

struct InstanceInput {
    @location(4) model0: vec4<f32>,
    @location(5) model1: vec4<f32>,
    @location(6) model2: vec4<f32>,
    @location(7) model3: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) world_position: vec3<f32>,
}

@vertex
fn vertex_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model_mat = mat4x4<f32>(
        instance.model0,
        instance.model1,
        instance.model2,
        instance.model3,
    );

    var transform = model_mat;
    var node_index = vertex.node_index;
    while node_index != 0xFFFFFFFF {
        let node = u_nodes[node_index];
        node_index = node.parent;
        transform *= node.transform;
    }

    let world_position = (transform * vec4(vertex.position, 1.0)).xyz;
    let clip_position = u_camera.mat_projection * u_camera.mat_view * vec4(world_position, 1.0);

    // We don't scale objects, so the model matrix without translation is good for now.
    let world_normal = (model_mat * vec4<f32>(vertex.normal, 0.0)).xyz;

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
