use crate::{
    engine::prelude::Renderer,
    game::scenes::world::{
        render_world::{Environment, RenderWorld},
        sim_world::SimWorld,
        systems::System,
    },
};

pub struct DayNightCycleSystem;

impl System for DayNightCycleSystem {
    fn update(&mut self, sim_world: &mut SimWorld, time: &super::Time) {
        sim_world.time_of_day += time.delta_time * 0.001;
    }

    fn extract(&mut self, sim_world: &SimWorld, render_world: &mut RenderWorld) {
        let time_of_day = sim_world.time_of_day;
        let source = &sim_world.day_night_cycle;

        let sun_dir = source.sun_dir.sample_sub_frame(time_of_day, true);
        let sun_color = source.sun_color.sample_sub_frame(time_of_day, true);

        let fog_distance = source.fog_distance.sample_sub_frame(time_of_day, true);
        let fog_near_fraction = source.fog_near_fraction.sample_sub_frame(time_of_day, true);
        let fog_color = source.fog_color.sample_sub_frame(time_of_day, true);

        render_world.environment = Environment {
            sun_dir: sun_dir.extend(0.0).to_array(),
            sun_color: sun_color.extend(1.0).to_array(),
            fog_color: fog_color.extend(1.0).to_array(),
            fog_distance,
            fog_near_fraction,
            _pad: Default::default(),
        };
    }

    fn prepare(&mut self, render_world: &mut RenderWorld, renderer: &Renderer) {
        renderer.queue.write_buffer(
            &render_world.environment_buffer,
            0,
            bytemuck::bytes_of(&render_world.environment),
        );
    }
}
