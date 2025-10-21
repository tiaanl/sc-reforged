use glam::{IVec2, UVec2, Vec4};

use crate::{
    engine::prelude::InputState,
    game::scenes::world::{sim_world::SimWorld, terrain::Terrain},
};

pub struct WorldInteractionSystem;

impl WorldInteractionSystem {
    pub fn input(&self, sim_world: &mut SimWorld, input_state: &InputState, viewport_size: UVec2) {
        sim_world.highlighted_chunks.clear();
        let _color = Vec4::new(1.0, 0.0, 0.0, 1.0);

        if let Some(mouse_position) = input_state.mouse_position() {
            let camera_ray_segment = sim_world
                .computed_camera
                .create_ray_segment(mouse_position.as_uvec2(), viewport_size);

            let mut terrain_chunks: Vec<IVec2> = Vec::default();

            sim_world
                .quad_tree
                .with_nodes_ray_segment(&camera_ray_segment, |node| {
                    if let Some(chunk_coord) = node.chunk_coord {
                        terrain_chunks.push(chunk_coord);
                    }
                });

            for chunk_coord in terrain_chunks {
                let lod = if let Some(chunk) = sim_world.terrain.chunk_at(chunk_coord) {
                    let center = chunk.bounding_box.center();
                    Terrain::calculate_lod(
                        sim_world.computed_camera.position,
                        sim_world.computed_camera.forward,
                        sim_world.camera.far,
                        center,
                    )
                } else {
                    continue;
                };

                if !sim_world
                    .terrain
                    .chunk_intersect_ray_segment(chunk_coord, &camera_ray_segment, Some(lod))
                    .is_empty()
                {
                    sim_world.highlighted_chunks.insert(chunk_coord);
                }
            }
        }
    }
}
