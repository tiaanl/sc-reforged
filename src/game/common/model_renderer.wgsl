#import world::camera
#import world::environment
#import world::geometry_buffers

@group(0) @binding(0) var<uniform> u_camera: camera::Camera;

@group(1) @binding(0) var<uniform> u_environment: environment::Environment;

@group(2) @binding(0) var t_texture: texture_2d<f32>;
@group(2) @binding(1) var s_sampler: sampler;

struct Node {
    transform: mat4x4<f32>,
    parent: u32,
}

@group(3) @binding(0) var<storage> u_nodes: array<Node>;

struct VertexInput {
    @location(0)
    position: vec3<f32>,

    @location(1)
    normal: vec3<f32>,

    @location(2)
    tex_coord: vec2<f32>,

    @location(3)
    node_index: u32,
}

struct InstanceInput {
    @location(4)
    col0: vec4<f32>,

    @location(5)
    col1: vec4<f32>,

    @location(6)
    col2: vec4<f32>,

    @location(7)
    col3: vec4<f32>,

    @location(8)
    entity_id: u32,
}

struct VertexOutput {
    @builtin(position)
    position: vec4<f32>,

    @location(0)
    tex_coord: vec2<f32>,

    @location(1)
    world_position: vec3<f32>,

    @location(2)
    normal: vec3<f32>,

    @location(3)
    entity_id: u32,
}

const ROOT_NODE: u32 = 0xFFFFFFFF;

@vertex
fn vertex(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model_mat = mat4x4<f32>(
        instance.col0,
        instance.col1,
        instance.col2,
        instance.col3,
    );

    let entity_id = instance.entity_id;

    // Apply the transform of the node tree until we hit the root.
    var node_index = vertex.node_index;
    var transform = model_mat;
    while (node_index != ROOT_NODE) {
        transform = transform * u_nodes[node_index].transform;
        node_index = u_nodes[node_index].parent;
    }

    let world_position = transform * vec4<f32>(vertex.position, 1.0);

    let normal_mat = mat3x3<f32>(
        transform[0].xyz,
        transform[1].xyz,
        transform[2].xyz,
    );
    let world_normal = normalize(normal_mat * vertex.normal);

    let clip_position = u_camera.mat_projection * u_camera.mat_view * world_position;

    return VertexOutput(
        clip_position,
        vertex.tex_coord,
        world_position.xyz,
        world_normal,
        entity_id,
    );
}

@fragment
fn fragment_opaque(vertex: VertexOutput) -> geometry_buffers::OpaqueGeometryBuffers {
    let base_color = textureSample(t_texture, s_sampler, vertex.tex_coord);

    if base_color.a < 1e-4 {
        discard;
    }

    let world_position = vertex.world_position;
    let world_normal = vertex.normal;

    let distance = length(world_position - u_camera.position);

    let diffuse = environment::diffuse_with_fog(
        u_environment,
        world_normal.xyz,
        base_color.rgb,
        distance,
    );

    return geometry_buffers::OpaqueGeometryBuffers(
        vec4<f32>(diffuse, 1.0),  // color
        vec4<f32>(world_position, 1.0),  // world_position
        vertex.entity_id,
    );
}

@fragment
fn fragment_alpha(vertex: VertexOutput) -> geometry_buffers::AlphaGeometryBuffers {
    let base_color = textureSample(t_texture, s_sampler, vertex.tex_coord);

    if base_color.a < 1e-4 {
        discard;
    }

    return geometry_buffers::AlphaGeometryBuffers(
        vertex.entity_id,
    );
}
