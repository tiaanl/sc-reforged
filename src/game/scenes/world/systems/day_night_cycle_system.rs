use bevy_ecs::prelude::*;

use crate::game::scenes::world::{sim_world::SimWorldState, systems::Time};

pub fn increment_time_of_day(mut state: ResMut<SimWorldState>, time: Res<Time>) {
    state.time_of_day += time.delta_time * 0.001;
}
