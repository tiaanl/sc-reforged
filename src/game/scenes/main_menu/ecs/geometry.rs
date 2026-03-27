use bevy_ecs::prelude::*;

use crate::{engine::storage::Handle, game::scenes::main_menu::window_renderer::TiledGeometry};

// #[derive(Component)]
// pub struct Geometry {}

#[derive(Component)]
pub struct GeometryTiled {
    pub tiled_geometry_handle: Handle<TiledGeometry>,
    pub alpha: f32,
    pub size: glam::UVec2,
}
