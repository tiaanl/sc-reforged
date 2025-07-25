#define_import_path world::terrain

const CELLS_PER_CHUNK: u32 = 8;

struct TerrainData {
    size: vec2<u32>,
    nominal_edge_size: f32,
    altitude_map_height_base: f32,
    water_level: f32,
    water_trans_depth: f32,
    water_trans_low: f32,
    water_trans_high: f32,
}

fn get_chunk_pos_from_index(terrain_data: TerrainData, chunk_index: u32) -> vec2<u32> {
    let terrain_chunks_x = terrain_data.size.x / CELLS_PER_CHUNK;
    let x = chunk_index % terrain_chunks_x;

    let terrain_chunks_y = terrain_data.size.y / CELLS_PER_CHUNK;
    let y = chunk_index / terrain_chunks_y;

    return vec2<u32>(x, y);
}
