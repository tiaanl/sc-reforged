use bevy_ecs::prelude::*;
use glam::Vec3;

use crate::game::scenes::world::{
    sim_world::{DayNightCycle, SimWorldState},
    systems::Time,
};

use super::RenderSnapshot;

pub fn extract_environment(
    mut snapshot: ResMut<RenderSnapshot>,
    day_night_cycle: Res<DayNightCycle>,
    time: Res<Time>,
    state: Res<SimWorldState>,
) {
    // TODO: Remove this from SimWorldSate.
    let tod = state.time_of_day;

    let env = &mut snapshot.environment;

    env.sim_time = time.sim_time;

    env.sun_dir = day_night_cycle.sun_dir.sample_sub_frame(tod, true);
    env.sun_color = day_night_cycle.sun_color.sample_sub_frame(tod, true);
    env.ambient_color = Vec3::splat(0.3);

    env.fog_color = day_night_cycle.fog_color.sample_sub_frame(tod, true);
    env.fog_distance = day_night_cycle.fog_distance.sample_sub_frame(tod, true);
    env.fog_near_fraction = day_night_cycle
        .fog_near_fraction
        .sample_sub_frame(tod, true);
}
