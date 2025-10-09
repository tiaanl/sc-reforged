use crate::game::scenes::world::{sim_world::SimWorld, systems::Time};

pub fn increment_time_of_day(sim_world: &mut SimWorld, time: &Time) {
    sim_world.time_of_day += time.delta_time * 0.001;
}
