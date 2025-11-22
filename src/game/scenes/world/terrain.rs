use glam::{IVec2, UVec2, Vec3};

use crate::{
    engine::storage::Handle,
    game::{
        image::Image,
        math::{BoundingBox, RaySegment, RayTriangleHit, triangle_intersect_ray_segment},
        scenes::world::height_map::HeightMap,
    },
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
    pub _min_node: IVec2,
    pub _max_node: IVec2,
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
        let chunk_dim = UVec2::new(
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

    pub fn chunk_intersect_ray_segment(
        &self,
        chunk_coord: IVec2,
        ray_segment: &RaySegment,
    ) -> Option<RayTriangleHit> {
        let chunk = self.chunk_at(chunk_coord).unwrap();

        let height_map = &self.height_map;

        const INDICES: [IVec2; 4] = [
            IVec2::ZERO,      // bottom-right
            IVec2::new(1, 0), // bottom-left
            IVec2::ONE,       // top-left
            IVec2::new(0, 1), // top-right
        ];

        let mut closest: Option<RayTriangleHit> = None;

        for y in chunk._min_node.y..chunk._max_node.y {
            for x in chunk._min_node.x..chunk._max_node.x {
                let node_coord = IVec2::new(x, y);
                let vertices =
                    INDICES.map(|offset| height_map.world_position_at_node(node_coord + offset));

                if let Some(hit) = triangle_intersect_ray_segment(
                    vertices[0],
                    vertices[1],
                    vertices[2],
                    ray_segment,
                    true,
                ) {
                    if let Some(closest_hit) = &closest {
                        if hit.t < closest_hit.t {
                            closest = Some(hit);
                        }
                    } else {
                        closest = Some(hit);
                    }
                }
                if let Some(hit) = triangle_intersect_ray_segment(
                    vertices[2],
                    vertices[3],
                    vertices[0],
                    ray_segment,
                    true,
                ) {
                    if let Some(closest_hit) = &closest {
                        if hit.t < closest_hit.t {
                            closest = Some(hit);
                        }
                    } else {
                        closest = Some(hit);
                    }
                }
            }
        }

        closest
    }

    pub fn calculate_lod(
        camera_position: Vec3,
        camera_forward: Vec3,
        camera_far: f32,
        chunk_center: Vec3,
    ) -> u32 {
        let far = camera_far.max(1e-6);
        let inv_step = Self::LOD_MAX as f32 / far;

        let forward_distance = (chunk_center - camera_position)
            .dot(camera_forward)
            .max(0.0);

        let t = forward_distance * inv_step;

        (t.floor() as i32).clamp(0, (Self::LOD_MAX - 1) as i32) as u32
    }

    fn build_chunks(height_map: &HeightMap, chunk_dim: UVec2) -> Vec<Chunk> {
        let mut chunks = Vec::with_capacity(chunk_dim.x as usize * chunk_dim.y as usize);

        for y in 0..chunk_dim.y {
            for x in 0..chunk_dim.x {
                let min_node =
                    UVec2::new(x * Self::CELLS_PER_CHUNK, y * Self::CELLS_PER_CHUNK).as_ivec2();
                let max_node = min_node + IVec2::splat(Self::CELLS_PER_CHUNK as i32);

                let mut min = height_map.world_position_at_node(min_node);
                let mut max = height_map.world_position_at_node(max_node);

                for yy in min_node.y..=max_node.y {
                    for xx in min_node.x..=max_node.x {
                        let altitude = height_map.node_at(IVec2::new(xx, yy)).w;
                        min.z = min.z.min(altitude);
                        max.z = max.z.max(altitude);
                    }
                }

                chunks.push(Chunk {
                    _min_node: min_node,
                    _max_node: max_node,
                    bounding_box: BoundingBox { min, max },
                })
            }
        }

        chunks
    }
}
