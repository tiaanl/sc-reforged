use crate::{
    engine::storage::Handle,
    game::{image::Image, scenes::world::new_height_map::NewHeightMap},
};

pub struct NewTerrain {
    pub height_map: NewHeightMap,
    pub terrain_texture: Handle<Image>,
    // pub water_image: Option<Handle<Image>>,
}

impl NewTerrain {
    /// Maximum level of detail downsampling. cell_count = (1 << LOD_MAX)
    pub const LOD_MAX: u32 = 3;
    /// Amount of cells in a chunk.
    pub const CELLS_PER_CHUNK: u32 = 8;
    /// Amount of nodes in a chunk.
    pub const NODES_PER_CHUNK: u32 = Self::CELLS_PER_CHUNK + 1;

    pub fn new(height_map: NewHeightMap, terrain_texture: Handle<Image>) -> Self {
        Self {
            height_map,
            terrain_texture,
        }
    }
}
