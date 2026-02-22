use bevy_ecs::resource::Resource;
use glam::{IVec2, UVec2, Vec2, Vec3};

use crate::{
    engine::storage::Handle,
    game::{
        image::Image,
        math::{BoundingBox, RaySegment, RayTriangleHit, triangle_intersect_ray_segment},
    },
};

use super::{height_map::HeightMap, quad_tree};

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

#[derive(Resource)]
pub struct Terrain {
    pub height_map: HeightMap,
    pub chunk_dim: UVec2,
    pub terrain_texture: Handle<Image>,
    pub strata_texture: Handle<Image>,
    // pub water_image: Option<Handle<Image>>,
    pub quad_tree: quad_tree::QuadTree,
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

    pub fn new(
        height_map: HeightMap,
        terrain_texture: Handle<Image>,
        strata_texture: Handle<Image>,
    ) -> Self {
        let chunk_dim = UVec2::new(
            height_map.size.x.next_multiple_of(Self::CELLS_PER_CHUNK) / Self::CELLS_PER_CHUNK,
            height_map.size.y.next_multiple_of(Self::CELLS_PER_CHUNK) / Self::CELLS_PER_CHUNK,
        );

        let min_max = Self::build_chunk_min_max(&height_map, chunk_dim);
        let quad_tree = quad_tree::QuadTree::build(
            chunk_dim,
            Vec2::splat(Self::CELLS_PER_CHUNK as f32 * height_map.cell_size),
            &min_max,
        );

        Self {
            height_map,
            chunk_dim,
            terrain_texture,
            strata_texture,
            quad_tree,
        }
    }

    pub fn chunk_bounding_box(&self, coord: IVec2) -> Option<BoundingBox> {
        if coord.x < 0 || coord.y < 0 {
            return None;
        }
        let coord = coord.as_uvec2();
        if coord.x >= self.chunk_dim.x || coord.y >= self.chunk_dim.y {
            return None;
        }

        Some(self.quad_tree.node_bounding_box(0, coord.x, coord.y))
    }

    /// Calculate the LOD for a chunk coordinate based on its bounds center.
    pub fn chunk_lod(
        &self,
        coord: IVec2,
        camera_position: Vec3,
        camera_forward: Vec3,
        camera_far: f32,
    ) -> Option<u32> {
        let bounding_box = self.chunk_bounding_box(coord)?;
        let center = bounding_box.center();

        Some(Self::calculate_lod(
            camera_position,
            camera_forward,
            camera_far,
            center,
        ))
    }

    pub fn _chunk_intersect_ray_segment(
        &self,
        chunk_coord: IVec2,
        ray_segment: &RaySegment,
    ) -> Option<RayTriangleHit> {
        if chunk_coord.x < 0 || chunk_coord.y < 0 {
            return None;
        }

        let chunk_coord = chunk_coord.as_uvec2();
        if chunk_coord.x >= self.chunk_dim.x || chunk_coord.y >= self.chunk_dim.y {
            return None;
        }

        let min_node = (chunk_coord * Self::CELLS_PER_CHUNK).as_ivec2();
        let max_node = min_node + IVec2::splat(Self::CELLS_PER_CHUNK as i32);

        let height_map = &self.height_map;

        const INDICES: [IVec2; 4] = [
            IVec2::ZERO,      // bottom-right
            IVec2::new(1, 0), // bottom-left
            IVec2::ONE,       // top-left
            IVec2::new(0, 1), // top-right
        ];

        let mut closest: Option<RayTriangleHit> = None;

        for y in min_node.y..max_node.y {
            for x in min_node.x..max_node.x {
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

    fn build_chunk_min_max(height_map: &HeightMap, chunk_dim: UVec2) -> Vec<quad_tree::MinMax> {
        let area = chunk_dim.x as usize * chunk_dim.y as usize;
        let mut min_max = Vec::with_capacity(area);

        for chunk_y in 0..chunk_dim.y {
            for chunk_x in 0..chunk_dim.x {
                let min_node = UVec2::new(chunk_x, chunk_y) * Self::CELLS_PER_CHUNK;
                let max_node = min_node + UVec2::splat(Self::CELLS_PER_CHUNK);

                let mut min_z = f32::INFINITY;
                let mut max_z = f32::NEG_INFINITY;

                for node_y in min_node.y..=max_node.y {
                    for node_x in min_node.x..=max_node.x {
                        let altitude = height_map
                            .node_at(IVec2::new(node_x as i32, node_y as i32))
                            .w;
                        min_z = min_z.min(altitude);
                        max_z = max_z.max(altitude);
                    }
                }

                min_max.push(quad_tree::MinMax(min_z, max_z));
            }
        }

        min_max
    }
}
