use bevy_ecs::prelude::*;

use crate::{engine::storage::Handle, game::render::textures::Texture};

// #[derive(Component)]
// pub struct Geometry {}

#[derive(Component)]
pub struct GeometryTiled {
    pub texture: Handle<Texture>,
    pub alpha: f32,
    pub size: glam::UVec2,
}
