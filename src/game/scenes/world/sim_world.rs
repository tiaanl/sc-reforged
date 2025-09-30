use glam::Vec3;

use crate::game::{
    animations::track::Track,
    camera::Camera,
    math::{Frustum, ViewProjection},
};

/// Holds data for the sun and fog values throughout the day and night.
#[derive(Default)]
pub struct DayNightCycle {
    pub sun_dir: Track<Vec3>,
    pub sun_color: Track<Vec3>,

    pub fog_distance: Track<f32>,
    pub fog_near_fraction: Track<f32>,
    pub fog_color: Track<Vec3>,
}

#[derive(Default)]
pub struct ComputedCamera {
    pub view_proj: ViewProjection,
    pub frustum: Frustum,
    pub position: Vec3,
    pub forward: Vec3,
}

/// Holds all the data for the world we are simulating.
pub struct SimWorld {
    pub cameras: [Camera; Self::CAMERA_COUNT],
    pub computed_cameras: [ComputedCamera; Self::CAMERA_COUNT],

    pub time_of_day: f32,
    pub day_night_cycle: DayNightCycle,
}

impl SimWorld {
    pub const CAMERA_COUNT: usize = 2;
    pub const MAIN_CAMERA_INDEX: usize = 0;
    pub const DEBUG_CAMERA_INDEX: usize = 1;
}
