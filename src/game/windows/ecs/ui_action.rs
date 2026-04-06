use std::{borrow::Cow, path::PathBuf};

use bevy_ecs::prelude::*;

use crate::game::{
    config::{configs::Configs, help_window_defs::HelpWindowDefs},
    windows::help_window::spawn_help_window,
};

#[derive(Clone, Debug, Message, PartialEq, Eq)]
pub enum UiAction {
    /// Exit the game.
    Exit,
    /// Show a help window with the given name.
    ShowHelpWindow(Cow<'static, str>),
}

pub fn handle_ui_actions(
    mut reader: MessageReader<UiAction>,
    configs: Res<Configs>,
    mut commands: Commands,
) {
    for message in reader.read() {
        match message {
            UiAction::Exit => println!("exit game!"),

            UiAction::ShowHelpWindow(name) => {
                if let Ok(help_window_defs) = configs
                    .load::<HelpWindowDefs>(PathBuf::from("config").join("help_window_defs.txt"))
                {
                    if let Some(help_def) = help_window_defs.get(name) {
                        tracing::info!("Show a help window with defintion: {name}");
                        spawn_help_window(&mut commands, help_def);
                    } else {
                        tracing::warn!("Help definition not found: {name}");
                    }
                } else {
                    tracing::warn!("Could not load HelpWindowDefs");
                }
            }
        }
    }
}
