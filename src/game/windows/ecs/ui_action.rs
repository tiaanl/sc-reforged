use bevy_ecs::prelude::*;

use crate::game::windows::help_window::spawn_help_window;

#[derive(Clone, Copy, Debug, Message, PartialEq, Eq)]
pub enum UiAction {
    /// Exit the game.
    Exit,
    /// Show the exit game confirmation.
    ShowExitConfirmation,
}

pub fn handle_ui_actions(mut commands: Commands, mut reader: MessageReader<UiAction>) {
    for message in reader.read() {
        match message {
            UiAction::Exit => println!("exit game!"),
            UiAction::ShowExitConfirmation => {
                spawn_help_window(&mut commands);
            }
        }
    }
}
