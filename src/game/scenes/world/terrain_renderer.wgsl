@group(0) @binding(0) var<uniform> u_camera: world::camera::Camera;

@group(1) @binding(0) var<uniform> u_environment: environment::Environment;

@group(2) @binding(0) var t_shadow_maps: texture_depth_2d_array;
@group(2) @binding(1) var s_shadow_maps: sampler_comparison;
@group(2) @binding(2) var<uniform> u_cascades: shadows::Cascades;

struct TerrainData {
    cell_count: vec2<u32>,
    chunk_count: vec2<u32>,
    nominal_edge_size: f32,
    _pad: vec3<u32>,
    water_level: f32,
    water_trans_depth: f32,
    water_trans_low: f32,
    water_trans_high: f32,
}

@group(3) @binding(0) var<uniform> u_terrain_data: TerrainData;
@group(3) @binding(1) var<storage, read> u_nodes: array<vec4<f32>>;
@group(3) @binding(3) var t_terrain: texture_2d<f32>;
@group(3) @binding(4) var t_water: texture_2d<f32>;
@group(3) @binding(5) var s_sampler: sampler;

const CELLS_PER_CHUNK: u32 = 8u;
const VERTICES_PER_CHUNK: u32 = CELLS_PER_CHUNK + 1u;

fn index_to_coord(index: u32, width: u32) -> vec2<u32> {
    return vec2<u32>(index % width, index / width);
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
}

@vertex
fn vertex_terrain(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let chunk_coord = index_to_coord(instance_index, u_terrain_data.chunk_count.x);
    let node_coord = index_to_coord(vertex_index, VERTICES_PER_CHUNK);
    let abs_node_coord = chunk_coord * CELLS_PER_CHUNK + node_coord;

    let node_index = abs_node_coord.y * (u_terrain_data.cell_count.x + 1) + abs_node_coord.x;
    let normal = u_nodes[node_index].xyz;
    let elevation = u_nodes[node_index].w;

    // Position of the chunk in world space.
    let chunk_origin = vec2<f32>(
        f32(chunk_coord.x * CELLS_PER_CHUNK) * u_terrain_data.nominal_edge_size,
        f32(chunk_coord.y * CELLS_PER_CHUNK) * u_terrain_data.nominal_edge_size,
    );

    // Position of the cell in chunk space.
    let cell_origin = vec2<f32>(
        f32(node_coord.x) * u_terrain_data.nominal_edge_size,
        f32(node_coord.y) * u_terrain_data.nominal_edge_size,
    );

    // The world position is where the chunk starts + where the cell inside the chunk starts.
    let world_position = vec3<f32>(chunk_origin + cell_origin, elevation);

    let clip_position = u_camera.mat_proj_view * vec4<f32>(world_position, 1.0);

    let tex_coord = vec2<f32>(
        f32(abs_node_coord.x) / f32(u_terrain_data.cell_count.x),
        f32(abs_node_coord.y) / f32(u_terrain_data.cell_count.y),
    );

    return VertexOutput(clip_position, world_position, normal, tex_coord);
}

@fragment
fn fragment_terrain(vertex: VertexOutput) -> world::geometry_buffers::OpaqueGeometryBuffers {
    let base_color = textureSample(t_terrain, s_sampler, vertex.tex_coord).rgb;

    var visibility = 1.0;

    for (var cascade_index = 0u; cascade_index < u_cascades.count; cascade_index += 1) {
        let light_ndc_position = shadows::project_to_light_ndc(
            u_cascades.cascades[cascade_index],
            vertex.world_position,
        );

        if math::inside_ndc(light_ndc_position) {
            // Map the clip position [-1..1] to [0..1].
            let shadow_uv = light_ndc_position.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);

            let bias = shadows::depth_bias(vertex.normal, u_environment.sun_dir.xyz);
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

    let distance_to_camera = length(u_camera.position.xyz - vertex.world_position);

    let lit = environment::diffuse_with_fog_shadow(
        u_environment,
        vertex.normal,
        base_color,
        distance_to_camera,
        visibility,
    );

    return world::geometry_buffers::to_opaque_geometry_buffer(lit, vertex.world_position, 0xFFFF0000);
}
