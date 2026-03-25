use bevy_ecs::prelude::*;

use crate::{engine::storage::Handle, game::scenes::main_menu::window_renderer::Texture};

// #[derive(Component)]
// pub struct Geometry {}

#[derive(Component)]
pub struct GeometryTiled {
    pub texture: Handle<Texture>,
    pub alpha: f32,
    pub size: glam::UVec2,
}
