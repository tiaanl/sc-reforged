use glam::{IVec2, UVec2, Vec4};

use crate::{
    engine::{gizmos::create_axis, prelude::InputState},
    game::scenes::world::{sim_world::SimWorld, systems::gizmo_system::GizmoSystem},
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

            let mut terrain_chunks = Vec::default();

            sim_world
                .quad_tree
                .with_nodes_ray_segment(&camera_ray_segment, |node| {
                    if let Some(chunk_coord) = node.chunk_coord {
                        terrain_chunks.push(chunk_coord);
                    }
                });

            for chunk_coord in terrain_chunks {
                if !sim_world
                    .terrain
                    .chunk_intersect_ray_segment(chunk_coord, &camera_ray_segment)
                    .is_empty()
                {
                    sim_world.highlighted_chunks.insert(chunk_coord);
                }
            }
        }
    }
}
