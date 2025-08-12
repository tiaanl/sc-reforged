#import world::animation
#import world::camera
#import world::environment
#import world::geometry_buffers

@group(0) @binding(0) var<uniform> u_camera: camera::Camera;

@group(1) @binding(0) var<uniform> u_environment: environment::Environment;

@group(2) @binding(0) var t_textures: binding_array<texture_2d<f32>>;
@group(2) @binding(1) var s_sampler: sampler;

@group(3) @binding(0) var t_positions: texture_2d<f32>;
@group(3) @binding(1) var t_rotations: texture_2d<f32>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
    @interpolate(flat) @location(3) node_index: u32,
    @interpolate(flat) @location(4) texture_index: u32,
};

struct InstanceInput {
    @location(5) col0: vec4<f32>,
    @location(6) col1: vec4<f32>,
    @location(7) col2: vec4<f32>,
    @location(8) col3: vec4<f32>,
    @interpolate(flat) @location(9) entity_id: u32,
    @location(10) time: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) normal: vec3<f32>,
    @interpolate(flat) @location(3) entity_id: u32,
    @interpolate(flat) @location(4) texture_index: u32,
};

@vertex
fn vertex_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.col0,
        instance.col1,
        instance.col2,
        instance.col3,
    );

    let local_transform = animation::local_transform_for_time(
        t_positions,
        t_rotations,
        vertex.node_index,
        instance.time,
    );
    let world_transform = model_matrix * local_transform;

    let world_position = world_transform * vec4<f32>(vertex.position, 1.0);
    let normal_matrix = mat3x3<f32>(
        world_transform[0].xyz,
        world_transform[1].xyz,
        world_transform[2].xyz,
    );
    let world_normal = math::normalize_safe(normal_matrix * vertex.normal);

    let view_projection = u_camera.mat_projection * u_camera.mat_view;
    let clip_position = view_projection * world_position;

    return VertexOutput(
        clip_position,
        vertex.tex_coord,
        world_position.xyz,
        world_normal,
        instance.entity_id,
        vertex.texture_index,
    );
}

@vertex
fn shadow_vertex(vertex: VertexInput, instance: InstanceInput) -> @builtin(position) vec4<f32> {
    let model_matrix = mat4x4<f32>(
        instance.col0,
        instance.col1,
        instance.col2,
        instance.col3,
    );

    let local_transform = animation::local_transform_for_time(
        t_positions,
        t_rotations,
        vertex.node_index,
        instance.time,
    );
    let world_transform = model_matrix * local_transform;

    let world_position = world_transform * vec4<f32>(vertex.position, 1.0);
    let normal_matrix = mat3x3<f32>(
        world_transform[0].xyz,
        world_transform[1].xyz,
        world_transform[2].xyz,
    );
    let world_normal = math::normalize_safe(normal_matrix * vertex.normal);

    let view_projection = u_camera.mat_projection * u_camera.mat_view;
    let clip_position = view_projection * world_position;

    return clip_position;
}

fn lit_color(vertex: VertexOutput) -> vec4<f32> {
    //let base_color = textureSample(t_texture, s_sampler, vertex.tex_coord);
    //let base_color = vec4<f32>(0.5, 0.5, 0.5, 1.0);

    let texture = t_textures[vertex.texture_index];
    let base_color = textureSample(texture, s_sampler, vertex.tex_coord);

    let camera_distance = length(vertex.world_position - u_camera.position);

    let diffuse = environment::diffuse_with_fog(
        u_environment,
        vertex.normal.xyz,
        base_color.rgb,
        camera_distance
    );

    return vec4<f32>(diffuse, base_color.a);
}

@fragment
fn fragment_opaque(vertex: VertexOutput) -> geometry_buffers::GeometryBuffers {
    let color = lit_color(vertex);

    if (color.a < math::EPSILON) {
        discard;
    }

    return geometry_buffers::to_geometry_buffer(
        vec4<f32>(color.rgb, 1.0),
        vertex.world_position,
        vertex.entity_id,
    );
}

@fragment
fn fragment_alpha(vertex: VertexOutput) -> geometry_buffers::GeometryBuffers {
    let color = lit_color(vertex);

    return geometry_buffers::to_geometry_buffer(
        color,
        vertex.world_position,
        vertex.entity_id,
    );
}