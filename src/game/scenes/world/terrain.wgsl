#import world::camera::Camera;
#import environment::diffuse_with_fog;
#import world::geometry_buffers::{
    AlphaGeometryBuffers,
    OpaqueGeometryBuffers,
    to_alpha_geometry_buffer,
    to_opaque_geometry_buffer,
};

@group(0) @binding(0) var<uniform> u_camera: Camera;
@group(1) @binding(0) var<uniform> u_environment: environment::Environment;

@group(2) @binding(0) var<storage, read> u_height_map: array<vec4<f32>>;
@group(2) @binding(1) var<uniform> u_terrain_data: terrain::TerrainData;
@group(2) @binding(2) var t_terrain_texture: texture_2d<f32>;
@group(2) @binding(3) var t_water_texture: texture_2d<f32>;
@group(2) @binding(4) var s_sampler: sampler;

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

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
}

@vertex
fn vertex_terrain(
    @builtin(instance_index) chunk_index: u32,
    @builtin(vertex_index) node_index: u32,
) -> VertexOutput {
    let node = terrain::get_node(u_terrain_data, chunk_index, node_index);
    let world_position = get_node_world_position(node);
    let normal = get_node_normal(node);

    let clip_position = u_camera.mat_proj_view * vec4(world_position, 1.0);

    let tex_coord = vec2<f32>(
        f32(node.x) / f32(u_terrain_data.size.x),
        f32(node.y) / f32(u_terrain_data.size.y),
    );

    return VertexOutput(
        clip_position,
        world_position,
        normal,
        tex_coord,
    );
}

@vertex
fn vertex_water(
    @builtin(instance_index) chunk_index: u32,
    @builtin(vertex_index) node_index: u32,
) -> VertexOutput {
    let node = terrain::get_node(u_terrain_data, chunk_index, node_index);
    let world_position = get_node_world_position(node);

    // Clip uses the actual water surface height
    let water_position = vec3<f32>(world_position.xy, u_terrain_data.water_level);
    let clip_position = u_camera.mat_proj_view * vec4<f32>(water_position, 1.0);

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
    let distance_to_camera = length(u_camera.position - world_position);

    let diffuse = diffuse_with_fog(u_environment, normal, base_color, distance_to_camera);

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
    let distance_to_camera = length(u_camera.position - world_position);

    let diffuse = diffuse_with_fog(u_environment, vertex.normal, base_color, distance_to_camera);

    return to_alpha_geometry_buffer(
        diffuse,
        opacity,
        opacity, // Not quite the correct weight here, but better than nothing.
    );
}

@vertex
fn vertex_wireframe(
    @builtin(instance_index) chunk_index: u32,
    @builtin(vertex_index) node_index: u32,
) -> @builtin(position) vec4<f32> {
    let node = terrain::get_node(u_terrain_data, chunk_index, node_index);
    let world_position = get_node_world_position(node);

    return u_camera.mat_proj_view * vec4<f32>(world_position, 1.0);
}

@fragment
fn fragment_wireframe() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 0.0, 1.0);
}
