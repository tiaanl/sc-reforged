#import world::camera
#import world::animation

@group(0) @binding(0) var<uniform> u_cascades: array<mat4x4<f32>, shadows::MAX_CASCADES>;

@group(1) @binding(0) var t_positions: texture_2d<f32>;
@group(1) @binding(1) var t_rotations: texture_2d<f32>;

var<push_constant> cascade_index: u32;

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

@vertex
fn vertex_shadow(vertex: VertexInput, instance: InstanceInput) -> @builtin(position) vec4<f32> {
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

    let world_position = model_matrix * local_transform * vec4<f32>(vertex.position, 1.0);

    return u_cascades[cascade_index] * world_position;
}
