use glam::IVec2;

use crate::game::scenes::world::sim_world::SimWorld;

/// Calculate visible elements for the current frame.
#[derive(Default)]
pub struct CullSystem {}

impl CullSystem {
    pub fn calculate_visible_chunks(&mut self, sim_world: &mut SimWorld) {
        //let frustum = &sim_world.computed_cameras[sim_world.active_camera as usize].frustum;

        sim_world.visible_chunks.clear();
        sim_world.visible_objects.clear();

        for y in 0..sim_world.terrain.chunk_dim.y as i32 {
            for x in 0..sim_world.terrain.chunk_dim.x as i32 {
                sim_world.visible_chunks.push(IVec2::new(x, y));
            }
        }

        sim_world.visible_objects = sim_world
            .objects
            .objects
            .iter()
            .map(|(handle, _)| handle)
            .collect();
    }
}
