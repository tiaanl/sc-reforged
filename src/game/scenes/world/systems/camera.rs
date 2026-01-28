use bevy_ecs::prelude::*;

use crate::game::scenes::world::sim_world::{Camera, ComputedCamera, DayNightCycle, SimWorldState};

pub fn compute_cameras(mut cameras: Query<(&Camera, &mut ComputedCamera)>) {
    for (camera, mut computed_camera) in cameras.iter_mut() {
        *computed_camera = camera.compute();
    }
}

pub fn update_far_distance(
    mut cameras: Query<&mut Camera>,
    day_night_cycle: Res<DayNightCycle>,
    state: Res<SimWorldState>,
) {
    let far = day_night_cycle
        .fog_distance
        .sample_sub_frame(state.time_of_day, true);

    for mut camera in cameras.iter_mut() {
        camera.far = far;
    }
}
