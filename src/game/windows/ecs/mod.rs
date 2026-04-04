use bevy_ecs::prelude::*;

use crate::engine::input::MouseButton;

pub mod button;
pub mod geometry;
pub mod rect;
pub mod render;
pub mod ui_action;
pub mod widgets;
pub mod window;

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

#[derive(Component, Default)]
pub struct ZIndex(pub i32);
