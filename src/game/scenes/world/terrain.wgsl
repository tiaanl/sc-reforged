#import environment::{diffuse_with_fog, diffuse_with_fog_shadow};
#import math::{position_in_frustum, project};
#import world::camera::Camera;
#import world::geometry_buffers::{
    AlphaGeometryBuffers,
    OpaqueGeometryBuffers,
    to_alpha_geometry_buffer,
    to_opaque_geometry_buffer,
};
#import terrain::ChunkInstance;
#import shadows::Cascades;

@group(0) @binding(0) var<uniform> u_camera: Camera;

@group(1) @binding(0) var<uniform> u_environment: environment::Environment;

@group(2) @binding(0) var<storage, read> u_height_map: array<vec4<f32>>;
@group(2) @binding(1) var<storage, read> u_chunk_instances: array<ChunkInstance>;
@group(2) @binding(2) var<uniform> u_terrain_data: terrain::TerrainData;
@group(2) @binding(3) var t_terrain_texture: texture_2d<f32>;
@group(2) @binding(4) var t_water_texture: texture_2d<f32>;
@group(2) @binding(5) var s_sampler: sampler;

@group(3) @binding(0) var t_shadow_maps: texture_depth_2d_array;
@group(3) @binding(1) var s_shadow_maps: sampler_comparison;
@group(3) @binding(2) var<uniform> u_cascades: Cascades;

fn get_node_normal_and_height(abs_node_coord: vec2<u32>) -> vec4<f32> {
    let index = abs_node_coord.y * (u_terrain_data.size.x + 1) + abs_node_coord.x;
    return u_height_map[index];
}

fn stitched_normal_and_height(
    chunk_lod: u32,
    chunk_coord: vec2<u32>,
    node_coord: vec2<u32>,
    abs_node_coord: vec2<u32>,
) -> vec4<f32> {
    var normal_and_height = get_node_normal_and_height(abs_node_coord);

    let last = terrain::CELLS_PER_CHUNK >> chunk_lod;

    // If last is one, the amount of cells in this chunk is 1, so no stitching is required.
    if last == 1u {
       return normal_and_height;
    }

    let scale = 1u << chunk_lod;
    let chunks_size = u_terrain_data.size / terrain::CELLS_PER_CHUNK;

    // Check if neighbors are valid.
    let has_neg_x = chunk_coord.x > 0u;
    let has_pos_x = (chunk_coord.x + 1u) < chunks_size.x;
    let has_neg_y = chunk_coord.y > 0u;
    let has_pos_y = (chunk_coord.y + 1u) < chunks_size.y;

    // -X
    if node_coord.x == 0u && has_neg_x {
        let nidx = (chunk_coord.y * chunks_size.x) + (chunk_coord.x - 1u);
        if u_chunk_instances[nidx].lod > chunk_lod && (node_coord.y & 1u) == 1u {
            let a = get_node_normal_and_height(abs_node_coord - vec2<u32>(0u, scale));
            let b = get_node_normal_and_height(abs_node_coord + vec2<u32>(0u, scale));
            normal_and_height = vec4<f32>(normalize(a.xyz + b.xyz), 0.5 * (a.w + b.w));
        }
    }

    // +X
    if node_coord.x == last && has_pos_x {
        let nidx = (chunk_coord.y * chunks_size.x) + (chunk_coord.x + 1u);
        if u_chunk_instances[nidx].lod > chunk_lod && (node_coord.y & 1u) == 1u {
            let a = get_node_normal_and_height(abs_node_coord - vec2<u32>(0u, scale));
            let b = get_node_normal_and_height(abs_node_coord + vec2<u32>(0u, scale));
            normal_and_height = vec4<f32>(normalize(a.xyz + b.xyz), 0.5 * (a.w + b.w));
        }
    }

    // -Y
    if node_coord.y == 0u && has_neg_y {
        let nidx = ((chunk_coord.y - 1u) * chunks_size.x) + chunk_coord.x;
        if u_chunk_instances[nidx].lod > chunk_lod && (node_coord.x & 1u) == 1u {
            let a = get_node_normal_and_height(abs_node_coord - vec2<u32>(scale, 0u));
            let b = get_node_normal_and_height(abs_node_coord + vec2<u32>(scale, 0u));
            normal_and_height = vec4<f32>(normalize(a.xyz + b.xyz), 0.5 * (a.w + b.w));
        }
    }

    // +Y
    if node_coord.y == last && has_pos_y {
        let nidx = ((chunk_coord.y + 1u) * chunks_size.x) + chunk_coord.x;
        if u_chunk_instances[nidx].lod > chunk_lod && (node_coord.x & 1u) == 1u {
            let a = get_node_normal_and_height(abs_node_coord - vec2<u32>(scale, 0u));
            let b = get_node_normal_and_height(abs_node_coord + vec2<u32>(scale, 0u));
            normal_and_height = vec4<f32>(normalize(a.xyz + b.xyz), 0.5 * (a.w + b.w));
        }
    }

    return normal_and_height;
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
}

@vertex
fn vertex_terrain(
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    let chunk = u_chunk_instances[instance_index];

    let chunk_coord = terrain::get_chunk_coord_from_instance_index(u_terrain_data, instance_index);
    let node_coord = terrain::get_node_coord_from_vertex_index(vertex_index);

    let scale = 1u << chunk.lod;
    let abs_node_coord = chunk_coord * terrain::CELLS_PER_CHUNK + node_coord * scale;

    let stitched = stitched_normal_and_height(
        chunk.lod,
        chunk_coord,
        node_coord,
        abs_node_coord,
    );

    let world_position = vec3<f32>(
        f32(abs_node_coord.x) * u_terrain_data.nominal_edge_size,
        f32(abs_node_coord.y) * u_terrain_data.nominal_edge_size,
        stitched.w,
    );

    let node_normal = stitched.xyz;

    let clip_position = u_camera.mat_proj_view * vec4(world_position, 1.0);

    let tex_coord = vec2<f32>(
        f32(abs_node_coord.x) / f32(u_terrain_data.size.x),
        f32(abs_node_coord.y) / f32(u_terrain_data.size.y),
    );

    return VertexOutput(
        clip_position,
        world_position,
        node_normal,
        tex_coord,
    );
}

@vertex
fn vertex_water(
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    let chunk = u_chunk_instances[instance_index];

    let chunk_coord = terrain::get_chunk_coord_from_instance_index(u_terrain_data, instance_index);
    let node_coord = terrain::get_node_coord_from_vertex_index(vertex_index);

    let scale = 1u << chunk.lod;
    let abs_node_coord = chunk_coord * terrain::CELLS_PER_CHUNK + node_coord * scale;

    let node = terrain::get_node(u_terrain_data, instance_index, vertex_index);
    let world_position = vec3<f32>(
        f32(abs_node_coord.x) * u_terrain_data.nominal_edge_size,
        f32(abs_node_coord.y) * u_terrain_data.nominal_edge_size,
        u_terrain_data.water_level,
    );

    // Clip uses the actual water surface height
    let clip_position = u_camera.mat_proj_view * vec4<f32>(world_position, 1.0);

    // Simple tiling for now
    let tex_coord = vec2<f32>(f32(node.x) / 8.0, f32(node.y) / 8.0);

    return VertexOutput(
        clip_position,
        world_position,             // keep original heightmap z for depth calc later
        vec3<f32>(0.0, 0.0, 1.0),   // water surface normal
        tex_coord,
    );
}

const TERRAIN_ENTITY_ID: u32 = 1u << 16u;

@fragment
fn fragment_terrain(vertex: VertexOutput) -> OpaqueGeometryBuffers {
    let base_color = textureSample(t_terrain_texture, s_sampler, vertex.tex_coord).rgb;

    let normal = vertex.normal;
    let world_position = vertex.world_position;
    let distance_to_camera = length(u_camera.position.xyz - world_position);

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

    var diffuse = diffuse_with_fog_shadow(
        u_environment,
        normal,
        base_color,
        distance_to_camera,
        visibility,
    );

    return to_opaque_geometry_buffer(
        diffuse,
        world_position,
        TERRAIN_ENTITY_ID,
    );
}

@fragment
fn fragment_water(vertex: VertexOutput) -> AlphaGeometryBuffers {
    var water_depth = u_terrain_data.water_level - vertex.world_position.z;

    // If the height of the terrain is above the water, *NO* water should be rendered.
    if water_depth <= 0.0 {
        discard;
    }

    water_depth = clamp(water_depth / u_terrain_data.water_trans_depth, 0.0, 1.0);

    // Transparency at the surface (depth == 0) and at max depth (depth == water_trans_depth).
    let opacity = mix(
        u_terrain_data.water_trans_low,
        u_terrain_data.water_trans_high,
        water_depth,
    );

    let base_color = textureSample(t_water_texture, s_sampler, vertex.tex_coord).rgb;

    let world_position = vertex.world_position;
    let distance_to_camera = length(u_camera.position.xyz - world_position);

    let diffuse = diffuse_with_fog(
        u_environment,
        vertex.normal,
        base_color,
        distance_to_camera,
    );

    return to_alpha_geometry_buffer(
        diffuse,
        opacity,
        opacity, // Not quite the correct weight here, but better than nothing.
    );
}

@vertex
fn vertex_wireframe(
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index) vertex_index: u32,
) -> @builtin(position) vec4<f32> {
    let chunk = u_chunk_instances[instance_index];
    let chunk_coord = terrain::get_chunk_coord_from_instance_index(u_terrain_data, instance_index);
    let node_coord = terrain::get_node_coord_from_vertex_index(vertex_index);

    let scale = 1u << chunk.lod;
    let abs_node_coord = chunk_coord * terrain::CELLS_PER_CHUNK + node_coord * scale;

    let stitched = stitched_normal_and_height(
        chunk.lod,
        chunk_coord,
        node_coord,
        abs_node_coord,
    );


    var world_position = vec3<f32>(
        f32(abs_node_coord.x) * u_terrain_data.nominal_edge_size,
        f32(abs_node_coord.y) * u_terrain_data.nominal_edge_size,
        stitched.w,
    );

    return u_camera.mat_proj_view * vec4<f32>(world_position, 1.0);
}

@fragment
fn fragment_wireframe() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 1.0, 0.0, 1.0);
}
