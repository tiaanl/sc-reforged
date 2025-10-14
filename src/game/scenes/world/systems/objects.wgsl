struct CameraEnv {
    proj_view: mat4x4<f32>,
    frustum: array<vec4<f32>, 6>,
    position: vec4<f32>,
    forward: vec4<f32>,

    sun_dir: vec4<f32>,       // x, y, z, 0
    sun_color: vec4<f32>,     // r, g, b, 1
    ambient_color: vec4<f32>, // r, g, b, 1
    fog_color: vec4<f32>,     // r, g, b, 1
    fog_distance: f32,
    fog_near_fraction: f32,
}

@group(0) @binding(0)
var<uniform> u_camera_env: CameraEnv;

struct TextureData {
    bucket: u32,
    layer: u32,
}

@group(1) @binding(0) var u_texture_buckets: binding_array<texture_2d_array<f32>>;
@group(1) @binding(1) var<storage, read> u_texture_data: array<TextureData>;
@group(1) @binding(2) var u_texture_sampler: sampler;

struct Node {
    transform: mat4x4<f32>,
}

@group(2) @binding(0) var<storage, read> u_nodes: array<Node>;

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
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
    @location(3) texture_data_index: u32,
}

@vertex
fn vertex_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    let local_matrix = u_nodes[instance.first_node_index + vertex.node_index].transform;

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
    );
}

@fragment
fn fragment_opaque(vertex: VertexOutput) -> @location(0) vec4<f32> {
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

    return vec4<f32>(lit, 1.0);
}


/// Diffuse + ambient lighting, modulated by shadow visibility.
fn diffuse(
    env: CameraEnv,
    normal: vec3<f32>,
    base_color: vec3<f32>,
    visibility: f32,              // 0 = full shadow, 1 = fully lit
) -> vec3<f32> {
    let N = normalize(normal);
    let L = -normalize(env.sun_dir.xyz); // from fragment toward sun

    let n_dot_l = max(dot(N, L), 0.0);

    // Direct sunlight (scaled by visibility)
    let sun_light = env.sun_color.rgb * n_dot_l * visibility;

    // Ambient term (not shadowed)
    let ambient = env.ambient_color.rgb;
    let ambient_color = env.sun_color.rgb * ambient;

    let lighting = sun_light + ambient_color;

    return lighting * base_color;
}

/// Same as diffuse_with_fog(), but blends in a shadow term.
fn diffuse_with_fog(
    env: CameraEnv,
    normal: vec3<f32>,
    base_color: vec3<f32>,
    distance: f32,
    visibility: f32,
) -> vec3<f32> {
    let lit_color = diffuse(env, normal, base_color, visibility);

    let fog_near = env.fog_distance * env.fog_near_fraction;
    let fog_far = env.fog_distance;

    let fog_factor = linear_fog_factor(fog_near, fog_far, distance);

    return mix(lit_color, env.fog_color.rgb, fog_factor);
}

fn linear_fog_factor(fog_near: f32, fog_far: f32, distance: f32) -> f32 {
    return clamp((distance - fog_near) / (fog_far - fog_near), 0.0, 1.0);
}
