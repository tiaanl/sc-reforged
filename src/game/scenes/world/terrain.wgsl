#import world::camera
#import world::environment
#import world::geometry_buffers
#import world::terrain

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

struct Node {
    x: u32,
    y: u32,
    index: u32,
}

fn get_chunk_pos_from_index(chunk_index: u32) -> vec2<u32> {
    let terrain_chunks_x = u_terrain_data.size.x / terrain::CELLS_PER_CHUNK;
    let x = chunk_index % terrain_chunks_x;

    let terrain_chunks_y = u_terrain_data.size.y / terrain::CELLS_PER_CHUNK;
    let y = chunk_index / terrain_chunks_y;

    return vec2<u32>(x, y);
}

fn get_node_index(chunk_pos: vec2<u32>, vertex_pos: vec2<u32>) -> Node {
    let x = chunk_pos.x * terrain::CELLS_PER_CHUNK + vertex_pos.x;
    let y = chunk_pos.y * terrain::CELLS_PER_CHUNK + vertex_pos.y;
    let index = y * (u_terrain_data.size.x + 1) + x;

    return Node(x, y, index);
}

fn get_node_world_position(node: Node) -> vec3<f32> {
    return vec3<f32>(
        f32(node.x) * u_terrain_data.nominal_edge_size,
        f32(node.y) * u_terrain_data.nominal_edge_size,
        u_height_map[node.index].w,
    );
}

fn get_node_normal(node: Node) -> vec3<f32> {
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
    let chunk_pos = get_chunk_pos_from_index(chunk_index);
    let node = get_node_index(chunk_pos, vertex.index);
    let world_position = get_node_world_position(node);
    let normal = get_node_normal(node);

    let clip_position = u_camera.mat_projection * u_camera.mat_view * vec4(world_position, 1.0);

    let tex_coord = vec2<f32>(
        f32(node.x) / f32(u_terrain_data.size.x),
        f32(node.y) / f32(u_terrain_data.size.y),
    );

    return VertexOutput(
        clip_position,
        world_position,
        normal,
        tex_coord,
        chunk_pos,
        chunk_index,
    );
}

@vertex
fn water_vertex_main(@builtin(instance_index) chunk_index: u32, vertex: VertexInput) -> VertexOutput {
    let chunk_pos = get_chunk_pos_from_index(chunk_index);
    let node = get_node_index(chunk_pos, vertex.index);
    let world_position = get_node_world_position(node);

    // We calculate the clip position from the correct water location, but we keep the height map
    // z value in the world position so we can calculate the depth in the fragment shader.
    let water_position = vec3(world_position.xy, u_terrain_data.water_level);
    let clip_position = u_camera.mat_projection * u_camera.mat_view * vec4<f32>(water_position, 1.0);

    // TODO: Calculate water texture coordinates.
    let tex_coord = vec2<f32>(
        f32(node.x) / 8.0,
        f32(node.y) / 8.0,
    );

    return VertexOutput(
        clip_position,
        world_position,
        vec3<f32>(0.0, 0.0, 1.0), // normal
        tex_coord,
        chunk_pos,
        chunk_index,
    );
}

const TERRAIN_ENTITY_ID: u32 = 1 << 16;

@fragment
fn fragment_main(vertex: VertexOutput) -> geometry_buffers::GeometryBuffers {
    let base_color = textureSample(t_terrain_texture, s_sampler, vertex.tex_coord);

    let world_position = vertex.world_position;
    let world_normal = vertex.normal;

    let distance = length(world_position - u_camera.position);

    let diffuse = environment::diffuse_with_fog(
        u_environment,
        world_normal.xyz,
        base_color.rgb,
        distance,
    );

    let sun_proj_view = u_environment.sun_proj * u_environment.sun_view;
    let light_point = sun_proj_view * vec4<f32>(world_position, 1.0);
    let inverse_w = 1.0 / light_point.w;

    let shadow_uv = light_point.xy * inverse_w * 0.5 + 0.5;
    let shadow_z = light_point.z * inverse_w;

    var color = vec4<f32>(diffuse.rgb, 1.0);

    if (any(shadow_uv < vec2(0.0)) || any(shadow_uv > vec2(1.0))) {
        color = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    let bias = 0.0015;

    // 1 = lit, 0 = fully shadowed (with PCF this is a smooth value)
    let lit = textureSampleCompare(
        shadow_map,
        shadow_map_sampler,
        shadow_uv,
        shadow_z + bias,
    );

    if lit < 0.1 {
        color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    }

    return geometry_buffers::to_geometry_buffer(
        color,
        vertex.world_position,
        TERRAIN_ENTITY_ID,
    );
}

@fragment
fn water_fragment_main(vertex: VertexOutput) -> geometry_buffers::GeometryBuffers {
    let water_depth = u_terrain_data.water_level - vertex.world_position.z;
    if water_depth <= 0.0 {
        discard;
    }

    let base_color = textureSample(
        t_water_texture,
        s_sampler,
        vertex.tex_coord,
    );

    var n = clamp(water_depth / u_terrain_data.water_trans_depth, 0.0, 1.0);
    let alpha = u_terrain_data.water_trans_low + (u_terrain_data.water_trans_high - u_terrain_data.water_trans_low) * n;

    let world_position = vertex.world_position;
    let world_normal = vertex.normal;

    let distance = length(world_position - u_camera.position);

    let diffuse = environment::diffuse_with_fog(
        u_environment,
        world_normal.xyz,
        base_color.rgb,
        distance,
    );

    let sun_proj_view = u_environment.sun_proj * u_environment.sun_view;
    let light_point = sun_proj_view * vec4<f32>(world_position, 1.0);
    let inverse_w = 1.0 / light_point.w;

    let shadow_uv = light_point.xy * inverse_w * 0.5 + 0.5;
    let shadow_z = light_point.z * inverse_w;

    var color = vec4<f32>(diffuse.rgb, alpha);

    if (any(shadow_uv < vec2(0.0)) || any(shadow_uv > vec2(1.0))) {
        color = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    let bias = 0.0015;
    let lit = textureSampleCompare(
        shadow_map,
        shadow_map_sampler,
        shadow_uv,
        shadow_z + bias,
    );

    if lit < 0.1 {
        color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    }

    return geometry_buffers::to_geometry_buffer(
        color,
        vertex.world_position,
        TERRAIN_ENTITY_ID + 1,
    );
}

@vertex
fn wireframe_vertex_main(@builtin(instance_index) chunk_index: u32, vertex: VertexInput) ->  @builtin(position) vec4<f32> {
    let chunk_pos = get_chunk_pos_from_index(chunk_index);
    let node = get_node_index(u_chunk_index, vertex.index);
    let world_position = get_node_world_position(node);

    let clip_position = u_camera.mat_projection * u_camera.mat_view * vec4(world_position, 1.0);

    return clip_position;
}

@fragment
fn wireframe_fragment_main() -> @location(0) vec4<f32> {
    return vec4(1.0, 1.0, 0.0, 1.0);
}
