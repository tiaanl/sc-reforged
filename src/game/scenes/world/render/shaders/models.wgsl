#import camera_env::{CameraEnv, diffuse_with_fog};

const FLAGS_HIGHLIGHTED: u32 = 1 << 0;
const FLAGS_CUSTOM_POSE: u32 = 1 << 1;
const COLOR_KEYED: u32 = 1 << 0;

@group(0) @binding(0)
var<uniform> u_camera_env: CameraEnv;

struct TextureData {
    bucket: u32,
    layer: u32,
    flags: u32,
    _pad: u32,
}

@group(1) @binding(0) var u_texture_buckets: binding_array<texture_2d_array<f32>>;
@group(1) @binding(1) var<storage, read> u_texture_data: array<TextureData>;
@group(1) @binding(2) var u_texture_sampler: sampler;

struct Node {
    transform: mat4x4<f32>,
    parent_index: u32,
}

@group(2) @binding(0) var<storage, read> u_nodes: array<Node>;

@group(3) @binding(0) var<storage, read> u_custom_nodes: array<mat4x4<f32>>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
    @location(3) node_index: u32,
    @location(4) texture_data_index: u32,
}

struct InstanceInput {
    @location(5) model_mat_0: vec4<f32>,
    @location(6) model_mat_1: vec4<f32>,
    @location(7) model_mat_2: vec4<f32>,
    @location(8) model_mat_3: vec4<f32>,
    @location(9) first_node_index: u32,
    @location(10) flags: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
    @location(3) texture_data_index: u32,
    @location(4) flags: u32,
}

const NODE_SENTINEL: u32 = 0xFFFFFFFFu;

fn get_node_transform(first_index: u32, index: u32) -> mat4x4<f32> {
    var transform = mat4x4<f32>(
        vec4<f32>(1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0),
    );
    var current = first_index + index;
    loop {
        let node = u_nodes[current];
        transform = node.transform * transform;
        if node.parent_index == NODE_SENTINEL {
            break;
        }
        current = first_index + node.parent_index;
    }

    return transform;
}

@vertex
fn vertex_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    var local_matrix: mat4x4<f32>;

    if (instance.flags & FLAGS_CUSTOM_POSE) != 0 {
        local_matrix = u_custom_nodes[instance.first_node_index + vertex.node_index];
    } else {
        local_matrix = get_node_transform(
            instance.first_node_index,
            vertex.node_index,
        );
    }

    let model_matrix = mat4x4<f32>(
        instance.model_mat_0,
        instance.model_mat_1,
        instance.model_mat_2,
        instance.model_mat_3,
    ) * local_matrix;

    let normal_matrix = mat3x3<f32>(
        model_matrix[0].xyz,
        model_matrix[1].xyz,
        model_matrix[2].xyz,
    );

    let world_position = model_matrix * vec4<f32>(vertex.position, 1.0);
    let world_normal = normalize(normal_matrix * vertex.normal.xyz);
    let tex_coord = vertex.tex_coord;
    let texture_data_index = vertex.texture_data_index;

    let clip_position = u_camera_env.proj_view * world_position;

    return VertexOutput(
        clip_position,
        world_position.xyz,
        world_normal,
        tex_coord,
        texture_data_index,
        instance.flags,
    );
}

@fragment
fn fragment_opaque(vertex: VertexOutput) -> geometry_buffer::OpaqueGeometryBuffer {
    let texture_data = u_texture_data[vertex.texture_data_index];
    let texture = u_texture_buckets[texture_data.bucket];
    let base_color = textureSample(texture, u_texture_sampler, vertex.tex_coord, texture_data.layer);

    // Handle color keyed textures.
    if (texture_data.flags & COLOR_KEYED) != 0 {
        // We just go with black as being the keyed color.
        if base_color.r + base_color.g + base_color.b == 0.0 {
            discard;
        }
    }

    let distance = length(vertex.world_position - u_camera_env.position.xyz);

    let lit = diffuse_with_fog(
        u_camera_env,
        vertex.world_normal,
        base_color.rgb,
        distance,
        1.0,
    );

    if (vertex.flags & FLAGS_HIGHLIGHTED) != 0 {
        let h = highlight(vec4<f32>(lit, base_color.a));
        return geometry_buffer::to_opaque_geometry_buffer(h.rgb);
    } 

    return geometry_buffer::to_opaque_geometry_buffer(lit);
}

@fragment
fn fragment_alpha(vertex: VertexOutput) -> geometry_buffer::AlphaGeometryBuffer {
    let texture_data = u_texture_data[vertex.texture_data_index];
    let texture = u_texture_buckets[texture_data.bucket];
    let base_color = textureSample(texture, u_texture_sampler, vertex.tex_coord, texture_data.layer);

    let distance = length(vertex.world_position - u_camera_env.position.xyz);

    let lit = diffuse_with_fog(
        u_camera_env,
        vertex.world_normal,
        base_color.rgb,
        distance,
        1.0,
    );

    if (vertex.flags & FLAGS_HIGHLIGHTED) != 0 {
        let h = highlight(vec4<f32>(lit, base_color.a));
        return geometry_buffer::to_alpha_geometry_buffer(h.rgb, h.a, 1.0);
    } 

    return geometry_buffer::to_alpha_geometry_buffer(lit, base_color.a, 1.0);
}

fn highlight(color: vec4<f32>) -> vec4<f32> {
    const HIGHLIGHT_COLOR = vec4<f32>(1.0, 1.0, 1.0, 0.2);

    let intensity = sin(u_camera_env.sim_time * 3.0) * 0.5 + 0.5;

    let highlight_rgb = mix(color.rgb, HIGHLIGHT_COLOR.rgb, HIGHLIGHT_COLOR.a);
    let rgb = mix(color.rgb, highlight_rgb, intensity * intensity);

    return vec4<f32>(rgb, color.a);
}
