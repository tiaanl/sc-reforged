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

/*
angola            320 x 320   40 x 40
angola_2          288 x 288   36 x 36
angola_tutorial   160 x 160   20 x 20
caribbean         288 x 288   36 x 36
ecuador           288 x 288   36 x 36
kola              320 x 320   40 x 40
kola_2            320 x 320   40 x 40
peru              168 x 256   21 x 32
romania           256 x 256   32 x 32
training          64 x 64     8 x 8
*/
