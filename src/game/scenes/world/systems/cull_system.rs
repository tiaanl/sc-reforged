use bevy_ecs::prelude::*;

use crate::game::scenes::world::sim_world::{
    ComputedCamera, Objects, SimWorldState, ecs::ActiveCamera,
};

/// Calculate visible elements for the current frame.
pub fn calculate_visible_chunks(
    camera: Single<&ComputedCamera, With<ActiveCamera>>,
    mut state: ResMut<SimWorldState>,
    objects: Res<Objects>,
) {
    let frustum = &camera.frustum;

    {
        let visible_objects = &mut state.visible_objects;
        objects
            .static_bvh
            .objects_in_frustum(frustum, visible_objects);
    }
}
