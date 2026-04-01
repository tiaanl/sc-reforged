use bevy_ecs::prelude::*;

#[derive(Component)]
pub struct Widget {
    pub position: glam::Vec2,
    pub size: glam::UVec2,
}

#[derive(Component, Default)]
pub struct Button {
    pub hovered: bool,
    pub pressed: bool,
}
