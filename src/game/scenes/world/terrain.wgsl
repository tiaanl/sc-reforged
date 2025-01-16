#import world::camera

@group(0) @binding(0) var<uniform> u_camera: camera::Camera;

struct TerrainData {
    size: vec2<u32>,
    nominal_edge_size: f32,
    water_level: f32,
    water_trans_depth: f32,
    water_trans_low: f32,
    water_trans_high: f32,
}

@group(1) @binding(0) var<storage> u_height_map: array<f32>;
@group(1) @binding(1) var<uniform> u_terrain_data: TerrainData;
@group(1) @binding(2) var t_terrain_texture: texture_2d<f32>;
@group(1) @binding(3) var t_water_texture: texture_2d<f32>;
@group(1) @binding(4) var s_sampler: sampler;

var<push_constant> u_chunk_index: vec2<u32>;

struct VertexInput {
    @location(0) index: vec2<u32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
}

struct Node {
    x: u32,
    y: u32,
    index: u32,
}

fn get_node_index(chunk_index: vec2<u32>, vertex_index: vec2<u32>) -> Node {
    let x = chunk_index.x * 8 + vertex_index.x;
    let y = chunk_index.y * 8 + vertex_index.y;
    let index = y * (u_terrain_data.size.x + 1) + x;

    return Node(x, y, index);
}

fn get_node_world_position(node: Node) -> vec3<f32> {
    return vec3<f32>(
        f32(node.x) * u_terrain_data.nominal_edge_size,
        f32(node.y) * u_terrain_data.nominal_edge_size,
        u_height_map[node.index],
    );
}

@vertex
fn vertex_main(vertex: VertexInput) -> VertexOutput {
    let node = get_node_index(u_chunk_index, vertex.index);
    let world_position = get_node_world_position(node);

    let clip_position = u_camera.mat_projection * u_camera.mat_view * vec4(world_position, 1.0);

    let tex_coord = vec2<f32>(
        f32(node.x) / f32(u_terrain_data.size.x),
        f32(node.y) / f32(u_terrain_data.size.y),
    );

    return VertexOutput(
        clip_position,
        world_position,
        tex_coord,
    );
}

@vertex
fn water_vertex_main(vertex: VertexInput) -> VertexOutput {
    let node = get_node_index(u_chunk_index, vertex.index);
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
        tex_coord,
    );
}

@fragment
fn fragment_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_terrain_texture, s_sampler, vertex.tex_coord);
    return color;
}

@fragment
fn water_fragment_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_water_texture, s_sampler, vertex.tex_coord);

    let water_depth = u_terrain_data.water_level - vertex.world_position.z;
    if water_depth <= 0.0 {
        discard;
    }

    var n = clamp(water_depth / u_terrain_data.water_trans_depth, 0.0, 1.0);
    let alpha = u_terrain_data.water_trans_low + (u_terrain_data.water_trans_high - u_terrain_data.water_trans_low) * n;

    return vec4<f32>(color.rgb, alpha);
}

@vertex
fn wireframe_vertex_main(vertex: VertexInput) ->  @builtin(position) vec4<f32> {
    let node = get_node_index(u_chunk_index, vertex.index);
    let world_position = get_node_world_position(node);

    let clip_position = u_camera.mat_projection * u_camera.mat_view * vec4(world_position, 1.0);

    return clip_position;
}

@fragment
fn wireframe_fragment_main() -> @location(0) vec4<f32> {
    return vec4(1.0, 1.0, 0.0, 1.0);
}
