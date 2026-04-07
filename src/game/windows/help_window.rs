use bevy_ecs::prelude::*;
use glam::{IVec2, UVec2};

use crate::game::{config::help_window_defs::HelpDef, windows::ecs::rect::Rect};

use super::ecs::window::spawn_window;

/// Spawn a new help window, getting its data from the specified [HelpDef].
pub fn spawn_help_window(commands: &mut Commands, help_def: &HelpDef) {
    tracing::info!("Spawning help window from: {}", help_def.id);

    let rect = Rect::new(IVec2::new(100, 100), UVec2::new(400, 300));

    let _window_entity = spawn_window(commands, rect).id();
}
