use ahash::HashMap;
use bevy_ecs::prelude::*;
use glam::IVec2;

use crate::game::scenes::world::{
    extract::TerrainChunk,
    sim_world::{ComputedCamera, SimWorldState, Terrain, ecs::ActiveCamera},
};

use super::RenderSnapshot;

pub fn extract_terrain_snapshot(
    mut snapshot: ResMut<RenderSnapshot>,
    computed_camera: Single<&ComputedCamera, With<ActiveCamera>>,
    terrain: Res<Terrain>,
    state: Res<SimWorldState>,

    mut visible_chunks_cache: Local<Vec<IVec2>>,
    mut chunk_lod_cache: Local<HashMap<IVec2, u32>>,
) {
    chunk_lod_cache.clear();

    snapshot.terrain.chunks.clear();
    snapshot.terrain.strata.clear();
    snapshot.terrain.strata_side_count = [0; 4];

    let super::Camera {
        position,
        forward,
        far,
        ..
    } = snapshot.camera;

    let chunk_dim = terrain.chunk_dim;

    terrain
        .quad_tree
        .visible_chunks(&computed_camera.frustum, &mut visible_chunks_cache);

    for &visible_coord in visible_chunks_cache.iter() {
        let mut lod_at = |coord: IVec2| {
            if let Some(lod) = chunk_lod_cache.get(&coord) {
                return Some(*lod);
            }

            terrain
                .chunk_lod(coord, position, forward, far)
                .inspect(|&lod| {
                    chunk_lod_cache.insert(coord, lod);
                })
        };

        let center_lod = lod_at(visible_coord).expect("Center chunk is always valid!");

        let mut flags = 0_u32;

        const NEIGHBORS: [IVec2; 4] = [
            IVec2::new(0, 1),
            IVec2::new(-1, 0),
            IVec2::new(0, -1),
            IVec2::new(1, 0),
        ];

        let neighbors = NEIGHBORS.map(|offset| lod_at(visible_coord + offset));
        for (i, neighbor_lod) in neighbors.iter().enumerate() {
            // A higher LOD means the resolution is lower, so we check greater than here.
            if neighbor_lod.unwrap_or(center_lod) > center_lod {
                flags |= 1 << i;
            }
        }

        // Highlight the chunk.
        const HIGHLIGHT: u32 = 1 << 15;
        if state.highlighted_chunks.contains(&visible_coord) {
            flags |= HIGHLIGHT;
        }

        let chunk_instance = TerrainChunk {
            coord: visible_coord,
            lod: center_lod,
            flags,
        };

        snapshot.terrain.chunks.push(chunk_instance);

        const SOUTH: u32 = 0;
        const WEST: u32 = 1;
        const NORTH: u32 = 2;
        const EAST: u32 = 3;

        if visible_coord.x == 0 {
            let chunk_instance = TerrainChunk {
                flags: chunk_instance.flags | (EAST << 8),
                ..chunk_instance
            };
            snapshot.terrain.strata.push(chunk_instance);
            snapshot.terrain.strata_side_count[EAST as usize] += 1;
        } else if visible_coord.x == chunk_dim.x as i32 - 1 {
            let chunk_instance = TerrainChunk {
                flags: chunk_instance.flags | (WEST << 8),
                ..chunk_instance
            };
            snapshot.terrain.strata.push(chunk_instance);
            snapshot.terrain.strata_side_count[WEST as usize] += 1;
        }

        if visible_coord.y == 0 {
            let chunk_instance = TerrainChunk {
                flags: chunk_instance.flags | (SOUTH << 8),
                ..chunk_instance
            };
            snapshot.terrain.strata.push(chunk_instance);
            snapshot.terrain.strata_side_count[SOUTH as usize] += 1;
        } else if visible_coord.y == chunk_dim.y as i32 - 1 {
            let chunk_instance = TerrainChunk {
                flags: chunk_instance.flags | (NORTH << 8),
                ..chunk_instance
            };
            snapshot.terrain.strata.push(chunk_instance);
            snapshot.terrain.strata_side_count[NORTH as usize] += 1;
        }
    }

    snapshot
        .terrain
        .strata
        .sort_unstable_by_key(|instance| instance.flags >> 8 & 0b11);

    snapshot
        .terrain
        .chunks
        .sort_unstable_by_key(|instance| instance.lod);
}
