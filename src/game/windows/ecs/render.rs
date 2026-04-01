use bevy_ecs::prelude::*;
use glam::Vec4;

use crate::{
    engine::storage::Handle,
    game::{assets::sprites::Sprite3d, windows::window_renderer::Font},
};

#[derive(Component)]
pub struct SpriteRender {
    pub position: glam::Vec2,
    pub alpha: f32,
    pub sprite: Handle<Sprite3d>,
    pub frame: usize,
}

#[derive(Component)]
pub struct TextRender {
    pub position: glam::Vec2,
    pub text: String,
    pub font: Font,
    pub color: Option<Vec4>,
}
