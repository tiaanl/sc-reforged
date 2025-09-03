#define_import_path terrain

const CELLS_PER_CHUNK: u32 = 8u;
const VERTICES_PER_CHUNK: u32 = CELLS_PER_CHUNK + 1;

struct TerrainData {
    size: vec2<u32>,
    nominal_edge_size: f32,
    altitude_map_height_base: f32,
    water_level: f32,
    water_trans_depth: f32,
    water_trans_low: f32,
    water_trans_high: f32,
}

struct ChunkInstance {
    center: vec3<f32>,
    radius: f32,

    min_elevation: f32,
    max_elevation: f32,

    lod: u32,
    flags: u32,
}

fn ceil_div_u32(a: u32, b: u32) -> u32 {
    // assert!(b > 0);
    return (a + b - 1u) / b;
}

fn get_coord_from_index(index: u32, width: u32) -> vec2<u32> {
    if width == 0 {
        return vec2(0u, 0u);
    }

    return vec2(index % width, index / width);
}

fn get_chunk_coord_from_instance_index(terrain_data: TerrainData, chunk_index: u32) -> vec2<u32> {
    let width = ceil_div_u32(terrain_data.size.x, CELLS_PER_CHUNK);
    return get_coord_from_index(chunk_index, width);
}

fn get_node_coord_from_vertex_index(index: u32) -> vec2<u32> {
    return get_coord_from_index(index, VERTICES_PER_CHUNK);
}

struct Node {
    x: u32,
    y: u32,
    index: u32,
}

fn get_node(terrain_data: TerrainData, chunk_index: u32, node_index: u32) -> Node {
    let chunk_coord = get_chunk_coord_from_instance_index(terrain_data, chunk_index);
    let node_coord = get_node_coord_from_vertex_index(node_index);

    let x = chunk_coord.x * CELLS_PER_CHUNK + node_coord.x;
    let y = chunk_coord.y * CELLS_PER_CHUNK + node_coord.y;
    let index = y * (terrain_data.size.x + 1) + x;
    return Node(x, y, index);
}
