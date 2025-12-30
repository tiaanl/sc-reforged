use bevy_ecs::prelude::*;

use crate::game::scenes::world::sim_world::{Objects, SimWorldState, Terrain};

/// Calculate visible elements for the current frame.
pub fn calculate_visible_chunks(
    mut state: ResMut<SimWorldState>,
    terrain: Res<Terrain>,
    objects: Res<Objects>,
) {
    let frustum = {
        state.computed_cameras[state.active_camera as usize]
            .frustum
            .clone()
    };

    {
        let visible_chunks = &mut state.visible_chunks;
        terrain.quad_tree.visible_chunks(&frustum, visible_chunks);
    }

    {
        let visible_objects = &mut state.visible_objects;
        objects
            .static_bvh
            .objects_in_frustum(&frustum, visible_objects);
    }
}
