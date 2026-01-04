use ahash::HashMap;
use bevy_ecs::prelude::*;
use glam::IVec2;

use crate::{
    engine::storage::Handle,
    game::scenes::world::{
        render::{
            ModelRenderFlags, ModelRenderSnapshot, ModelToRender, TerrainRenderSnapshot, gpu,
        },
        sim_world::{
            Camera, ComputedCamera, Object, Objects, SimWorld, Terrain, ecs::ActiveCamera,
        },
    },
};

#[derive(Default)]
pub struct TerrainExtract {
    /// A cache for chunk LOD's cleared and rebuilt on each extract.
    chunk_lod_cache: HashMap<IVec2, u32>,

    /// A cache for a list of visible chunks calculated on each extract.
    pub visible_chunks_cache: Vec<IVec2>,
}

impl TerrainExtract {
    pub fn extract(&mut self, sim_world: &mut SimWorld, snapshot: &mut TerrainRenderSnapshot) {
        self.chunk_lod_cache.clear();

        let (camera, computed_camera) = {
            sim_world
                .ecs
                .query_filtered::<(&Camera, &ComputedCamera), With<ActiveCamera>>()
                .single(&sim_world.ecs)
                .unwrap()
        };

        let camera_position = computed_camera.position;
        let camera_forward = computed_camera.forward;
        let camera_far = camera.far;

        let chunk_instances = &mut snapshot.chunk_instances;
        let strata_instances = &mut snapshot.strata_instances;
        let strata_instances_side_count = &mut snapshot.strata_instances_side_count;

        chunk_instances.clear();
        strata_instances.clear();
        *strata_instances_side_count = [0; 4];

        let terrain = sim_world.ecs.resource::<Terrain>();
        let chunk_dim = terrain.chunk_dim;

        let state = sim_world.state();

        terrain
            .quad_tree
            .visible_chunks(&computed_camera.frustum, &mut self.visible_chunks_cache);
        for visible_coord in self.visible_chunks_cache.iter() {
            let mut lod_at = |coord: IVec2| {
                if let Some(lod) = self.chunk_lod_cache.get(&coord) {
                    return Some(*lod);
                }

                terrain
                    .chunk_lod(coord, camera_position, camera_forward, camera_far)
                    .inspect(|&lod| {
                        self.chunk_lod_cache.insert(coord, lod);
                    })
            };

            let center_lod = lod_at(*visible_coord).expect("Center chunk is always valid!");

            let mut flags = 0_u32;

            const NEIGHBORS: [IVec2; 4] = [
                IVec2::new(0, 1),
                IVec2::new(-1, 0),
                IVec2::new(0, -1),
                IVec2::new(1, 0),
            ];

            let neighbors = NEIGHBORS.map(|offset| lod_at(*visible_coord + offset));
            for (i, neighbor_lod) in neighbors.iter().enumerate() {
                // A higher LOD means the resolution is lower, so we check greater than here.
                if neighbor_lod.unwrap_or(center_lod) > center_lod {
                    flags |= 1 << i;
                }
            }

            // Highlight the chunk.
            const HIGHLIGHT: u32 = 1 << 15;
            if state.highlighted_chunks.contains(visible_coord) {
                flags |= HIGHLIGHT;
            }

            let chunk_instance = gpu::ChunkInstanceData {
                coord: visible_coord.as_uvec2().to_array(),
                lod: center_lod,
                flags,
            };

            chunk_instances.push(chunk_instance);

            const SOUTH: u32 = 0;
            const WEST: u32 = 1;
            const NORTH: u32 = 2;
            const EAST: u32 = 3;

            if visible_coord.x == 0 {
                let chunk_instance = gpu::ChunkInstanceData {
                    flags: chunk_instance.flags | (EAST << 8),
                    ..chunk_instance
                };
                strata_instances.push(chunk_instance);
                strata_instances_side_count[EAST as usize] += 1;
            } else if visible_coord.x == chunk_dim.x as i32 - 1 {
                let chunk_instance = gpu::ChunkInstanceData {
                    flags: chunk_instance.flags | (WEST << 8),
                    ..chunk_instance
                };
                strata_instances.push(chunk_instance);
                strata_instances_side_count[WEST as usize] += 1;
            }

            if visible_coord.y == 0 {
                let chunk_instance = gpu::ChunkInstanceData {
                    flags: chunk_instance.flags | (SOUTH << 8),
                    ..chunk_instance
                };
                strata_instances.push(chunk_instance);
                strata_instances_side_count[SOUTH as usize] += 1;
            } else if visible_coord.y == chunk_dim.y as i32 - 1 {
                let chunk_instance = gpu::ChunkInstanceData {
                    flags: chunk_instance.flags | (NORTH << 8),
                    ..chunk_instance
                };
                strata_instances.push(chunk_instance);
                strata_instances_side_count[NORTH as usize] += 1;
            }
        }

        strata_instances.sort_unstable_by_key(|instance| instance.flags >> 8 & 0b11);

        snapshot
            .chunk_instances
            .sort_unstable_by_key(|instance| instance.lod);
    }
}

#[derive(Default)]
pub struct ModelExtract {
    visible_objects_cache: Vec<Handle<Object>>,
}

impl ModelExtract {
    pub fn extract(&mut self, sim_world: &mut SimWorld, snapshot: &mut ModelRenderSnapshot) {
        snapshot.models.clear();

        let computed_camera = {
            sim_world
                .ecs
                .query_filtered::<&ComputedCamera, With<ActiveCamera>>()
                .single(&sim_world.ecs)
                .unwrap()
        };

        let objects = sim_world.ecs.resource::<Objects>();
        objects
            .static_bvh
            .objects_in_frustum(&computed_camera.frustum, &mut self.visible_objects_cache);

        let state = sim_world.state();
        let selected_objects = &state.selected_objects;

        self.visible_objects_cache
            .iter()
            .filter_map(|object_handle| objects.get(*object_handle).map(|o| (o, *object_handle)))
            .for_each(|(object, handle)| {
                let mut flags = ModelRenderFlags::empty();
                flags.set(
                    ModelRenderFlags::HIGHLIGHTED,
                    selected_objects.contains(&handle),
                );

                use crate::game::scenes::world::sim_world::ObjectData;

                let model = match &object.data {
                    ObjectData::Scenery { model }
                    | ObjectData::Biped { model, .. }
                    | ObjectData::SingleModel { model } => *model,
                };

                snapshot.models.push(ModelToRender {
                    model,
                    transform: object.transform.to_mat4(),
                    flags,
                });
            });
    }
}
