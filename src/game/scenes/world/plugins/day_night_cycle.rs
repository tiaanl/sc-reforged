use bevy_ecs::{
    resource::Resource,
    system::{Res, ResMut},
};
use glam::Vec3;

use crate::game::{
    scenes::world::sim_world::{Plugin, SimWorld, Time},
    track::Track,
};

pub struct DayNightCycleSnapshot {
    pub sun_dir: Vec3,
    pub sun_color: Vec3,

    pub fog_distance: f32,
    pub fog_near_fraction: f32,
    pub fog_color: Vec3,
}

/// Holds data for the sun and fog values throughout the day and night.
#[derive(Default)]
pub struct DayNightCycleData {
    pub sun_dir: Track<Vec3>,
    pub sun_color: Track<Vec3>,

    pub fog_distance: Track<f32>,
    pub fog_near_fraction: Track<f32>,
    pub fog_color: Track<Vec3>,
}

#[derive(Resource)]
pub struct DayNightCycle {
    pub time_of_day: f32,
    pub data: DayNightCycleData,
}

impl DayNightCycle {
    pub fn snapshot(&self) -> DayNightCycleSnapshot {
        DayNightCycleSnapshot {
            sun_dir: self.data.sun_dir.sample_sub_frame(self.time_of_day, true),
            sun_color: self.data.sun_color.sample_sub_frame(self.time_of_day, true),
            fog_distance: self
                .data
                .fog_distance
                .sample_sub_frame(self.time_of_day, true),
            fog_near_fraction: self
                .data
                .fog_near_fraction
                .sample_sub_frame(self.time_of_day, true),
            fog_color: self.data.fog_color.sample_sub_frame(self.time_of_day, true),
        }
    }
}

pub struct DayNightCyclePlugin {
    time_of_day: f32,
    data: DayNightCycleData,
}

impl DayNightCyclePlugin {
    pub fn new(time_of_day: f32, day_night_cycle_data: DayNightCycleData) -> Self {
        Self {
            time_of_day,
            data: day_night_cycle_data,
        }
    }
}

impl Plugin for DayNightCyclePlugin {
    fn init(self, sim_world: &mut SimWorld) {
        sim_world.world.insert_resource(DayNightCycle {
            time_of_day: self.time_of_day,
            data: self.data,
        });

        sim_world.update_schedule.add_systems(increment_time_of_day);
    }
}

fn increment_time_of_day(mut day_night_cycle: ResMut<DayNightCycle>, time: Res<Time>) {
    day_night_cycle.time_of_day += time.delta_time;
}
