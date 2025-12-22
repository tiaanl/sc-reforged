use glam::IVec2;

use crate::game::scenes::world::sim_world::SimWorld;

/// Calculate visible elements for the current frame.
#[derive(Default)]
pub struct CullSystem {}

impl CullSystem {
    pub fn calculate_visible_chunks(&mut self, sim_world: &mut SimWorld) {
        let frustum = &sim_world.computed_cameras[sim_world.active_camera as usize].frustum;

        sim_world.visible_chunks.clear();
        sim_world.visible_objects.clear();

        {
            sim_world
                .terrain
                .quad_tree
                .visible_chunks(frustum, &mut sim_world.visible_chunks);
        }

        sim_world.visible_objects = sim_world
            .objects
            .objects
            .iter()
            .map(|(handle, _)| handle)
            .collect();
    }
}
