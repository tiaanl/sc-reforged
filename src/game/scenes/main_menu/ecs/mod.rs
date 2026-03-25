use bevy_ecs::prelude::*;

use crate::{engine::storage::Handle, game::render::textures::Texture};

pub mod geometry;

#[derive(Component)]
pub struct Widget {
    pub position: glam::UVec2,
    pub size: glam::UVec2,
}

#[derive(Component)]
pub struct WidgetRenderer {
    pub texture: Handle<Texture>,
}
