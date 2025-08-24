#define_import_path terrain

const CELLS_PER_CHUNK: u32 = 8u;

struct TerrainData {
    size: vec2<u32>,
    nominal_edge_size: f32,
    altitude_map_height_base: f32,
    water_level: f32,
    water_trans_depth: f32,
    water_trans_low: f32,
    water_trans_high: f32,
}

fn ceil_div_u32(a: u32, b: u32) -> u32 {
    // assert!(b > 0);
    return (a + b - 1u) / b;
}

fn chunk_grid(size_in_cells: vec2<u32>) -> vec2<u32> {
    return vec2<u32>(
        ceil_div_u32(size_in_cells.x, CELLS_PER_CHUNK),
        ceil_div_u32(size_in_cells.y, CELLS_PER_CHUNK)
    );
}

fn get_chunk_pos_from_index(terrain_data: TerrainData, chunk_index: u32) -> vec2<u32> {
    let grid = chunk_grid(terrain_data.size);
    let width = max(grid.x, 1u);
    let total = grid.x * grid.y;

    let index = select(chunk_index, total - 1u, chunk_index >= total);

    let x = index % width;
    let y = index / width;

    return vec2<u32>(x, y);
}

struct Node {
    x: u32,
    y: u32,
    index: u32,
}

fn get_node(terrain_data: TerrainData, chunk_pos: vec2<u32>, vertex_pos: vec2<u32>) -> Node {
    let x = chunk_pos.x * CELLS_PER_CHUNK + vertex_pos.x;
    let y = chunk_pos.y * CELLS_PER_CHUNK + vertex_pos.y;
    let index = y * (terrain_data.size.x + 1) + x;
    return Node(x, y, index);
}
