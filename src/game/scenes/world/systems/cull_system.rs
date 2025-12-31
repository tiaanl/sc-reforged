use bevy_ecs::prelude::*;

use crate::game::scenes::world::sim_world::{
    ComputedCamera, Objects, SimWorldState, Terrain, ecs::ActiveCamera,
};

/// Calculate visible elements for the current frame.
pub fn calculate_visible_chunks(
    camera: Single<&ComputedCamera, With<ActiveCamera>>,
    mut state: ResMut<SimWorldState>,
    terrain: Res<Terrain>,
    objects: Res<Objects>,
) {
    let frustum = &camera.frustum;

    {
        let visible_chunks = &mut state.visible_chunks;
        terrain.quad_tree.visible_chunks(frustum, visible_chunks);
    }

    {
        let visible_objects = &mut state.visible_objects;
        objects
            .static_bvh
            .objects_in_frustum(frustum, visible_objects);
    }
}
