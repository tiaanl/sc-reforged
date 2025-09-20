#import shadows::Cascades;
#import world::animation
#import world::camera::Camera;
#import world::geometry_buffers

@group(0) @binding(0) var<uniform> u_camera: Camera;

@group(1) @binding(0) var<uniform> u_environment: environment::Environment;

@group(2) @binding(0) var texture_buckets: binding_array<texture_2d_array<f32>>;
@group(2) @binding(1) var<storage, read> texture_data: array<TextureData>;
@group(2) @binding(2) var texture_sampler: sampler;

@group(3) @binding(0) var t_positions: texture_2d<f32>;
@group(3) @binding(1) var t_rotations: texture_2d<f32>;

@group(4) @binding(0) var t_shadow_maps: texture_depth_2d_array;
@group(4) @binding(1) var s_shadow_maps: sampler_comparison;
@group(4) @binding(2) var<uniform> u_cascades: Cascades;

struct TextureData {
    bucket: u32,
    layer: u32,
}

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) tex_coord: vec2<f32>,
    @location(3) node_index: u32,
    @location(4) texture_data_index: u32,
};

struct InstanceInput {
    @location(5) col0: vec4<f32>,
    @location(6) col1: vec4<f32>,
    @location(7) col2: vec4<f32>,
    @location(8) col3: vec4<f32>,
    @location(9) entity_id: u32,
    @location(10) time: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) entity_id: u32,
    @location(4) texture_data_index: u32,
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

    let world_position = world_transform * vertex.position;
    let normal_matrix = mat3x3<f32>(
        world_transform[0].xyz,
        world_transform[1].xyz,
        world_transform[2].xyz,
    );
    let world_normal = math::normalize_safe(normal_matrix * vertex.normal.xyz);

    let view_projection = u_camera.mat_proj_view;
    let clip_position = view_projection * world_position;

    return VertexOutput(
        clip_position,
        vertex.tex_coord,
        world_position.xyz,
        world_normal,
        instance.entity_id,
        vertex.texture_data_index,
    );
}

fn lit_color(vertex: VertexOutput) -> vec4<f32> {
    let t_data = texture_data[vertex.texture_data_index];
    let base_color = textureSample(
        texture_buckets[t_data.bucket],
        texture_sampler,
        vertex.tex_coord,
        t_data.layer,
    );

    let world_position = vertex.world_position;
    let normal = vertex.normal;
    let camera_distance = length(world_position - u_camera.position.xyz);

    var visibility = 1.0;

    for (var cascade_index = 0u; cascade_index < u_cascades.count; cascade_index += 1) {
        let light_ndc_position = shadows::project_to_light_ndc(
            u_cascades.cascades[cascade_index],
            world_position,
        );

        if math::inside_ndc(light_ndc_position) {
            // Map the clip position [-1..1] to [0..1].
            let shadow_uv = light_ndc_position.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);

            let bias = shadows::depth_bias(normal, u_environment.sun_dir.xyz);
            let depth_ref = clamp(light_ndc_position.z, 0.0, 1.0);

            visibility = shadows::sample_shadow_pcf_3x3(
                t_shadow_maps,
                s_shadow_maps,
                cascade_index,
                shadow_uv,
                depth_ref,
            );

            break;
        }
    }


    let diffuse = environment::diffuse_with_fog_shadow(
        u_environment,
        vertex.normal.xyz,
        base_color.rgb,
        camera_distance,
        visibility,
    );

    return vec4<f32>(diffuse, base_color.a);
}

@fragment
fn fragment_opaque(vertex: VertexOutput) -> geometry_buffers::OpaqueGeometryBuffers {
    let color = lit_color(vertex);

    if (color.a < math::EPSILON) {
        discard;
    }

    return geometry_buffers::to_opaque_geometry_buffer(
        color.rgb,
        vertex.world_position,
        vertex.entity_id,
    );
}

@fragment
fn fragment_alpha(vertex: VertexOutput) -> geometry_buffers::AlphaGeometryBuffers {
    let color = lit_color(vertex);

    return geometry_buffers::to_alpha_geometry_buffer(
        color.rgb,
        color.a,
        1.0, // No weight for now.
    );
}