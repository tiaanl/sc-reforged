use bevy_ecs::prelude::*;
use glam::Mat4;

#[derive(Clone, Component, Debug, Default)]
pub struct Pose {
    pub bones: Vec<Mat4>,
}
