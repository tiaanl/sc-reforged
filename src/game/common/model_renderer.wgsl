#import world::camera
#import world::geometry_buffers

@group(0) @binding(0) var<uniform> u_camera: camera::Camera;

@group(1) @binding(0) var t_texture: texture_2d<f32>;
@group(1) @binding(1) var s_sampler: sampler;

var<push_constant> entity_id: u32;

struct Node {
    transform: mat4x4<f32>,
    parent: u32,
}

@group(2) @binding(0) var<storage> u_nodes: array<Node>;

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
    );
}

@fragment
fn fragment_opaque(vertex: VertexOutput) -> geometry_buffers::OpaqueGeometryBuffers {
    let base_color = textureSample(t_texture, s_sampler, vertex.tex_coord);

    if base_color.a < 1e-4 {
        discard;
    }

    return geometry_buffers::OpaqueGeometryBuffers(
        base_color,  // color
        vec4<f32>(vertex.world_position, 1.0),  // world_position
        vec4<f32>(vertex.normal, 1.0),  // normal
        entity_id,
    );
}

@fragment
fn fragment_alpha(vertex: VertexOutput) -> geometry_buffers::AlphaGeometryBuffers {
    let base_color = textureSample(t_texture, s_sampler, vertex.tex_coord);

    if base_color.a < 1e-4 {
        discard;
    }

    let alpha = base_color.a;
    let weight = max(0.01, alpha * 8.0);
    let accumulation = vec4<f32>(base_color.rgb * alpha, alpha) * weight;
    let revealage = alpha;

    return geometry_buffers::AlphaGeometryBuffers(accumulation, revealage, entity_id);
}
