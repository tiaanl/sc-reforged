#define_import_path world::terrain

const CELLS_PER_CHUNK: u32 = 8;

struct TerrainData {
    size: vec2<u32>,
    nominal_edge_size: f32,
    water_level: f32,
    water_trans_depth: f32,
    water_trans_low: f32,
    water_trans_high: f32,
}
