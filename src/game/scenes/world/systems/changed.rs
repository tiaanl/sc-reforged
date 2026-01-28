use bevy_ecs::prelude::*;

use crate::game::scenes::world::sim_world::SimWorldState;

pub fn time_of_day_changed(state: Res<SimWorldState>, mut last_tod: Local<f32>) -> bool {
    let changed = state.time_of_day != *last_tod;
    *last_tod = state.time_of_day;
    changed
}
