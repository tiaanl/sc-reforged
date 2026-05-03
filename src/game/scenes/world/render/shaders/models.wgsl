#import camera_env::{CameraEnv, diffuse_with_fog};

const FLAGS_HIGHLIGHTED: u32 = 1 << 0;
const FLAGS_CUSTOM_POSE: u32 = 1 << 1;

@group(0) @binding(0)
var<uniform> u_camera_env: CameraEnv;

@group(1) @binding(0) var u_texture: texture_2d<f32>;
@group(1) @binding(1) var u_sampler: sampler;

// Default-pose, precomposed bone transforms; one entry per bone in the model.
@group(2) @binding(0) var<storage, read> u_nodes: array<mat4x4<f32>>;

// Custom-pose composed bone transforms, packed contiguously across all custom-pose
// instances; `first_node_index` selects this instance's slice.
@group(3) @binding(0) var<storage, read> u_custom_nodes: array<mat4x4<f32>>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
    @location(3) node_index: u32,
}

struct InstanceInput {
    @location(4) model_mat_0: vec4<f32>,
    @location(5) model_mat_1: vec4<f32>,
    @location(6) model_mat_2: vec4<f32>,
    @location(7) model_mat_3: vec4<f32>,
    @location(8) first_node_index: u32,
    @location(9) flags: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
    @location(3) flags: u32,
}

@vertex
fn vertex_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    var local_matrix: mat4x4<f32>;

    if (instance.flags & FLAGS_CUSTOM_POSE) != 0 {
        local_matrix = u_custom_nodes[instance.first_node_index + vertex.node_index];
    } else {
        local_matrix = u_nodes[vertex.node_index];
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

    let clip_position = u_camera_env.proj_view * world_position;

    return VertexOutput(
        clip_position,
        world_position.xyz,
        world_normal,
        tex_coord,
        instance.flags,
    );
}

fn shade(vertex: VertexOutput, base_color: vec4<f32>) -> vec3<f32> {
    let distance = length(vertex.world_position - u_camera_env.position.xyz);

    return diffuse_with_fog(
        u_camera_env,
        vertex.world_normal,
        base_color.rgb,
        distance,
        1.0,
    );
}

@fragment
fn fragment_opaque(vertex: VertexOutput) -> geometry_buffer::OpaqueGeometryBuffer {
    let base_color = textureSample(u_texture, u_sampler, vertex.tex_coord);
    let lit = shade(vertex, base_color);

    if (vertex.flags & FLAGS_HIGHLIGHTED) != 0 {
        let h = highlight(vec4<f32>(lit, base_color.a));
        return geometry_buffer::to_opaque_geometry_buffer(h.rgb);
    }

    return geometry_buffer::to_opaque_geometry_buffer(lit);
}

@fragment
fn fragment_opaque_keyed(vertex: VertexOutput) -> geometry_buffer::OpaqueGeometryBuffer {
    let base_color = textureSample(u_texture, u_sampler, vertex.tex_coord);

    // Color-keyed: black is treated as transparent.
    if base_color.r + base_color.g + base_color.b == 0.0 {
        discard;
    }

    let lit = shade(vertex, base_color);

    if (vertex.flags & FLAGS_HIGHLIGHTED) != 0 {
        let h = highlight(vec4<f32>(lit, base_color.a));
        return geometry_buffer::to_opaque_geometry_buffer(h.rgb);
    }

    return geometry_buffer::to_opaque_geometry_buffer(lit);
}

@fragment
fn fragment_alpha(vertex: VertexOutput) -> geometry_buffer::AlphaGeometryBuffer {
    let base_color = textureSample(u_texture, u_sampler, vertex.tex_coord);
    let lit = shade(vertex, base_color);

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
