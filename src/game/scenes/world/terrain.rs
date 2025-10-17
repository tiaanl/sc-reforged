use glam::{IVec2, UVec2, ivec2, uvec2};

use crate::{
    engine::storage::Handle,
    game::{image::Image, math::BoundingBox, scenes::world::height_map::HeightMap},
};

// Size of each terrain:
//
//                     cells       chunks
//   angola            320 x 320   40 x 40
//   angola_2          288 x 288   36 x 36
//   angola_tutorial   160 x 160   20 x 20
//   caribbean         288 x 288   36 x 36
//   ecuador           288 x 288   36 x 36
//   kola              320 x 320   40 x 40
//   kola_2            320 x 320   40 x 40
//   peru              168 x 256   21 x 32
//   romania           256 x 256   32 x 32
//   training          64 x 64     8 x 8

pub struct Chunk {
    pub bounding_box: BoundingBox,
}

pub struct Terrain {
    pub height_map: HeightMap,
    pub chunk_dim: UVec2,
    pub chunks: Vec<Chunk>,
    pub terrain_texture: Handle<Image>,
    // pub water_image: Option<Handle<Image>>,
}

impl Terrain {
    /// Number of LOD levels.
    pub const LOD_COUNT: u32 = 4;
    /// Maximum level of detail downsampling. cell_count = (1 << LOD_MAX)
    pub const LOD_MAX: u32 = Self::LOD_COUNT - 1;
    /// Amount of cells in a chunk.
    pub const CELLS_PER_CHUNK: u32 = 1 << Self::LOD_MAX;
    /// Amount of nodes in a chunk.
    pub const NODES_PER_CHUNK: u32 = Self::CELLS_PER_CHUNK + 1;

    pub fn new(height_map: HeightMap, terrain_texture: Handle<Image>) -> Self {
        let chunk_dim = uvec2(
            height_map.size.x.next_multiple_of(Self::CELLS_PER_CHUNK) / Self::CELLS_PER_CHUNK,
            height_map.size.y.next_multiple_of(Self::CELLS_PER_CHUNK) / Self::CELLS_PER_CHUNK,
        );

        let chunks = Self::build_chunks(&height_map, chunk_dim);

        Self {
            height_map,
            chunk_dim,
            chunks,
            terrain_texture,
        }
    }

    pub fn chunk_at(&self, coord: IVec2) -> Option<&Chunk> {
        let coord = coord
            .clamp(IVec2::ZERO, self.chunk_dim.as_ivec2())
            .as_uvec2();
        let index = coord.y as usize * self.chunk_dim.x as usize + coord.x as usize;
        self.chunks.get(index)
    }

    fn build_chunks(height_map: &HeightMap, chunk_dim: UVec2) -> Vec<Chunk> {
        let mut chunks = Vec::with_capacity(chunk_dim.x as usize * chunk_dim.y as usize);

        for y in 0..chunk_dim.y {
            for x in 0..chunk_dim.x {
                let start = uvec2(x * Self::CELLS_PER_CHUNK, y * Self::CELLS_PER_CHUNK).as_ivec2();
                let end = start + IVec2::splat(Self::CELLS_PER_CHUNK as i32);

                let mut min = height_map.world_position_at(start);
                let mut max = height_map.world_position_at(end);

                for yy in start.y..=end.y {
                    for xx in start.x..=end.x {
                        let altitude = height_map.node_at(ivec2(xx, yy)).w;
                        min.z = min.z.min(altitude);
                        max.z = max.z.max(altitude);
                    }
                }

                chunks.push(Chunk {
                    bounding_box: BoundingBox { min, max },
                })
            }
        }

        chunks
    }
}
