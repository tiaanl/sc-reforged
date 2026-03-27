use bevy_ecs::prelude::*;

use crate::{engine::storage::Handle, game::assets::sprites::Sprite3d};

pub mod geometry;

#[derive(Component)]
pub struct Widget {
    pub position: glam::UVec2,
    pub size: glam::UVec2,
}

#[derive(Component)]
pub struct WidgetRenderer {
    pub sprite: Handle<Sprite3d>,
    pub frame: usize,
}
