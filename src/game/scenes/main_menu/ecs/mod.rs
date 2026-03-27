use bevy_ecs::prelude::*;

use crate::{engine::storage::Handle, game::assets::sprites::Sprite3d};

pub mod geometry;

#[derive(Component)]
pub struct Widget {
    pub position: glam::Vec2,
    pub size: glam::UVec2,
    pub alpha: f32,
}

#[derive(Component)]
pub struct WidgetRenderer {
    pub sprite: Handle<Sprite3d>,
    pub frame: usize,
}

#[derive(Component)]
pub struct MainMenuButton {
    pub base_position: glam::IVec2,
    pub size: glam::UVec2,
    pub button_offset: glam::IVec2,
    pub shadow_offset: glam::IVec2,
    pub shadow_entity: Entity,
    pub text_entity: Entity,
    pub hover_progress_ms: f32,
    pub hovered: bool,
}

#[derive(Message)]
pub enum WindowMessage {
    MouseMove(glam::UVec2),
    MouseLeave,
}
