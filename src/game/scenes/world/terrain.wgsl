#import world::camera
#import world::environment
#import world::geometry_buffers

@group(0) @binding(0) var<uniform> u_camera: camera::Camera;
@group(1) @binding(0) var<uniform> u_environment: environment::Environment;

@group(2) @binding(0) var<storage> u_height_map: array<vec4<f32>>;
@group(2) @binding(1) var<uniform> u_terrain_data: terrain::TerrainData;
@group(2) @binding(2) var t_terrain_texture: texture_2d<f32>;
@group(2) @binding(3) var t_water_texture: texture_2d<f32>;
@group(2) @binding(4) var s_sampler: sampler;
@group(2) @binding(5) var shadow_map: texture_depth_2d;
@group(2) @binding(6) var shadow_map_sampler: sampler_comparison;

var<push_constant> u_chunk_index: vec2<u32>;

fn get_node_world_position(node: terrain::Node) -> vec3<f32> {
    return vec3<f32>(
        f32(node.x) * u_terrain_data.nominal_edge_size,
        f32(node.y) * u_terrain_data.nominal_edge_size,
        u_height_map[node.index].w,
    );
}

fn get_node_normal(node: terrain::Node) -> vec3<f32> {
    return u_height_map[node.index].xyz;
}

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) index: vec2<u32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
    @location(3) chunk_pos: vec2<u32>,
    @location(4) chunk_index: u32,
}

@vertex
fn vertex_main(@builtin(instance_index) chunk_index: u32, vertex: VertexInput) -> VertexOutput {
    let chunk_pos = terrain::get_chunk_pos_from_index(u_terrain_data, chunk_index);
    let node = terrain::get_node(u_terrain_data, chunk_pos, vertex.index);
    let world_position = get_node_world_position(node);
    let normal = get_node_normal(node);

    let clip_position = u_camera.mat_proj_view * vec4(world_position, 1.0);

    let tex_coord = vec2<f32>(
        f32(node.x) / f32(u_terrain_data.size.x),
        f32(node.y) / f32(u_terrain_data.size.y),
    );

    return VertexOutput(clip_position, world_position, normal, tex_coord, chunk_pos, chunk_index);
}

@vertex
fn water_vertex_main(@builtin(instance_index) chunk_index: u32, vertex: VertexInput) -> VertexOutput {
    let chunk_pos = terrain::get_chunk_pos_from_index(u_terrain_data, chunk_index);
    let node = terrain::get_node(u_terrain_data, chunk_pos, vertex.index);
    let world_position = get_node_world_position(node);

    // Clip uses the actual water surface height
    let water_position = vec3(world_position.xy, u_terrain_data.water_level);
    let clip_position = u_camera.mat_proj_view * vec4<f32>(water_position, 1.0);

    // Simple tiling for now
    let tex_coord = vec2<f32>(f32(node.x) / 8.0, f32(node.y) / 8.0);

    return VertexOutput(
        clip_position,
        world_position,                // keep original heightmap z for depth calc later
        vec3<f32>(0.0, 0.0, 1.0),     // water surface normal
        tex_coord,
        chunk_pos,
        chunk_index,
    );
}

const TERRAIN_ENTITY_ID: u32 = 1u << 16u;

// ---------- Terrain ----------

@fragment
fn fragment_main(v: VertexOutput) -> geometry_buffers::OpaqueGeometryBuffers {
    let albedo = textureSample(t_terrain_texture, s_sampler, v.tex_coord).rgb;

    // World-space values
    let N = normalize(v.normal);
    let Vpos = v.world_position;
    let dist = length(u_camera.position - Vpos);

    // Sun vectors
    let Lrays = normalize(u_environment.sun_dir.xyz); // rays direction
    let L = -Lrays;                                   // direction to light
    let ndotl = max(dot(N, L), 0.0);

    // Shadow coords (proj*view) with Y flip (D3D/Vulkan texture space)
    let lp   = u_environment.sun_proj_view * vec4(Vpos, 1.0);
    let ndc  = lp.xyz / lp.w;
    let shadow_uv = ndc.xy * vec2(0.5, -0.5) + vec2(0.5, 0.5);
    let shadow_z  = ndc.z;

    var shadow = 1.0;
    if !(any(shadow_uv < vec2(0.0)) || any(shadow_uv > vec2(1.0))) {
        let bias = 0.0015;
        shadow = textureSampleCompare(shadow_map, shadow_map_sampler, shadow_uv, shadow_z + bias);
    }

    // Ambient + shadowed direct
    let ambient = 0.15 * albedo;
    let direct  = albedo * u_environment.sun_color.rgb * ndotl;
    var rgb = ambient + shadow * direct;

    // Fog
    let fog_near = u_environment.fog_params.x;
    let fog_far  = u_environment.fog_params.y;
    let fog_t = clamp((dist - fog_near) / (fog_far - fog_near), 0.0, 1.0);
    rgb = mix(rgb, u_environment.fog_color.rgb, fog_t);

    return geometry_buffers::to_opaque_geometry_buffer(
        rgb,
        Vpos,
        TERRAIN_ENTITY_ID,
    );
}

// ---------- Water ----------

@fragment
fn water_fragment_main(v: VertexOutput) -> geometry_buffers::AlphaGeometryBuffers {
    // --- Water opacity/transparency --- //

    let water_depth = u_terrain_data.water_level - v.world_position.z;

    // If the height of the terrain is above the water, *NO* water should be rendered.
    if water_depth <= 0.0 {
        discard;
    }

    let depth01 = clamp(water_depth / u_terrain_data.water_trans_depth, 0.0, 1.0);

    // Transparency at the surface (depth == 0) and at max depth (depth == water_trans_depth).
    let surface_opacity = u_terrain_data.water_trans_low;
    let deep_opacity    = u_terrain_data.water_trans_high;

    let opacity = mix(surface_opacity, deep_opacity, depth01);

    // --- Color --- //

    let base_color = textureSample(t_water_texture, s_sampler, v.tex_coord).rgb;

    // --- Shadow map --- //

    // Use the actual surface position for lighting & shadows
    let Vpos = vec3(v.world_position.xy, u_terrain_data.water_level);
    let N = normalize(v.normal); // (0,0,1) from vertex shader
    let dist = length(u_camera.position - Vpos);

    // Sun light
    let Lrays = normalize(u_environment.sun_dir.xyz);
    let L = -Lrays;
    let ndotl = max(dot(N, L), 0.0);

    // Shadows for water surface (same transform + Y flip)
    let lp   = u_environment.sun_proj_view * vec4(Vpos, 1.0);
    let ndc  = lp.xyz / lp.w;
    let shadow_uv = ndc.xy * vec2(0.5, -0.5) + vec2(0.5, 0.5);
    let shadow_z  = ndc.z;

    var shadow = 1.0;
    if !(any(shadow_uv < vec2(0.0)) || any(shadow_uv > vec2(1.0))) {
        let bias = 0.0015;
        shadow = textureSampleCompare(shadow_map, shadow_map_sampler, shadow_uv, shadow_z + bias);
    }

    // Ambient + shadowed direct, then fog; preserve alpha
    let ambient = 0.15 * base_color;
    let direct  = base_color * u_environment.sun_color.rgb * ndotl;
    var rgb = ambient + shadow * direct;

    let fog_near = u_environment.fog_params.x;
    let fog_far  = u_environment.fog_params.y;
    let fog_t = clamp((dist - fog_near) / (fog_far - fog_near), 0.0, 1.0);
    rgb = mix(rgb, u_environment.fog_color.rgb, fog_t);

    return geometry_buffers::to_alpha_geometry_buffer(
        rgb,
        opacity,
        1.0, // No weight right now.
    );
}

// ---------- Wireframe debug ----------

@vertex
fn wireframe_vertex_main(@builtin(instance_index) chunk_index: u32, vertex: VertexInput) -> @builtin(position) vec4<f32> {
    let chunk_pos = terrain::get_chunk_pos_from_index(u_terrain_data, chunk_index);
    let node = terrain::get_node(u_terrain_data, u_chunk_index, vertex.index);
    let world_position = get_node_world_position(node);
    return u_camera.mat_proj_view * vec4(world_position, 1.0);
}

@fragment
fn wireframe_fragment_main() -> @location(0) vec4<f32> {
    return vec4(1.0, 1.0, 0.0, 1.0);
}
