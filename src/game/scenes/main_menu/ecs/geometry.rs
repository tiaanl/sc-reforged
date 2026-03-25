use bevy_ecs::prelude::*;

use super::super::render::TextureId;

// #[derive(Component)]
// pub struct Geometry {}

#[derive(Component)]
pub struct GeometryTiled {
    pub texture_id: TextureId,
    pub alpha: f32,
    pub size: glam::UVec2,
}
