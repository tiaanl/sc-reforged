use bevy_ecs::prelude::*;
use glam::Vec4;

use crate::{
    engine::{input::MouseButton, storage::Handle},
    game::{assets::sprites::Sprite3d, scenes::main_menu::window_renderer::Font},
};

pub mod geometry;

#[derive(Component)]
pub struct Widget {
    pub position: glam::Vec2,
    pub size: glam::UVec2,
}

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

#[derive(Component, Default)]
pub struct Button {
    pub hovered: bool,
    pub pressed: bool,
}

#[derive(Component)]
pub struct MainMenuButtonAnimation {
    pub button_offset: glam::IVec2,
    pub shadow_offset: glam::IVec2,
    pub shadow_entity: Entity,
    pub text_entity: Entity,
    pub pressed_entity: Entity,
    pub hover_progress_ms: f32,
}

#[derive(Message)]
#[allow(clippy::enum_variant_names)]
pub enum WindowMessage {
    MouseMove(glam::UVec2),
    MouseLeave,
    MouseDown(MouseButton),
    MouseUp(MouseButton),
}

#[derive(Debug, Message)]
pub enum WidgetMessage {
    Enter(Entity),
    Exit(Entity),
    MouseDown(Entity, MouseButton),
    MouseUp(Entity, MouseButton),
}
