use bevy_ecs::prelude::*;

use crate::game::scenes::world::sim_world::{Camera, ComputedCamera};

pub fn compute_cameras(mut cameras: Query<(&Camera, &mut ComputedCamera)>) {
    for (camera, mut computed_camera) in cameras.iter_mut() {
        *computed_camera = camera.compute();
    }
}
