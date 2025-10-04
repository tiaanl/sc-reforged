use crate::game::scenes::world::systems::{System, UpdateContext};

pub struct DayNightCycleSystem;

impl System for DayNightCycleSystem {
    fn update(&mut self, context: &mut UpdateContext) {
        context.sim_world.time_of_day += context.time.delta_time * 0.001;
    }
}
