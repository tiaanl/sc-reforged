use bevy_ecs::prelude::*;
use glam::Mat4;

#[derive(Component, Default)]
pub struct Pose {
    pub bones: Vec<Mat4>,
}
