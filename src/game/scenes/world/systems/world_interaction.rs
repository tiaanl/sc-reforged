use glam::{UVec2, Vec4};

use crate::{engine::prelude::InputState, game::scenes::world::sim_world::SimWorld};

pub struct WorldInteractionSystem;

impl WorldInteractionSystem {
    pub fn input(&self, sim_world: &mut SimWorld, input_state: &InputState, viewport_size: UVec2) {
        sim_world.highlighted_chunks.clear();
        let _color = Vec4::new(1.0, 0.0, 0.0, 1.0);

        if let Some(mouse_position) = input_state.mouse_position() {
            let _camera_ray_segment = sim_world
                .computed_camera
                .create_ray_segment(mouse_position.as_uvec2(), viewport_size);
        }
    }
}
