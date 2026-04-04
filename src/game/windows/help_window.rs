use bevy_ecs::prelude::*;
use glam::{IVec2, UVec2};

use crate::game::windows::ecs::{rect::Rect, window::Window};

pub fn spawn_help_window(commands: &mut Commands) {
    tracing::info!("Spawning help window");
    commands.spawn((
        Window,
        Rect {
            position: IVec2::new(100, 100),
            size: UVec2::new(400, 300),
        },
    ));
}
