use bevy_ecs::prelude::*;
use glam::Vec3;

use crate::game::{config::Campaign, track::Track};

/// Holds data for the sun and fog values throughout the day and night.
#[derive(Default, Resource)]
pub struct DayNightCycle {
    pub sun_dir: Track<Vec3>,
    pub sun_color: Track<Vec3>,

    pub fog_distance: Track<f32>,
    pub fog_near_fraction: Track<f32>,
    pub fog_color: Track<Vec3>,
}

impl DayNightCycle {
    pub fn from_campaign(campaign: &Campaign) -> Self {
        let mut sun_dir = Track::default();
        let mut sun_color = Track::default();

        let mut fog_distance = Track::default();
        let mut fog_near_fraction = Track::default();
        let mut fog_color = Track::default();

        campaign
            .time_of_day
            .iter()
            .enumerate()
            .for_each(|(i, tod)| {
                let index = i as u32;

                sun_dir.insert(index, tod.sun_dir);
                sun_color.insert(index, tod.sun_color);

                fog_distance.insert(index, tod.fog_distance);
                fog_near_fraction.insert(index, tod.fog_near_fraction);
                fog_color.insert(index, tod.fog_color);
            });

        Self {
            sun_dir,
            sun_color,
            fog_distance,
            fog_near_fraction,
            fog_color,
        }
    }
}
