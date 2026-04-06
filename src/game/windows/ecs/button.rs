use bevy_ecs::prelude::*;

use crate::{
    engine::input::MouseButton,
    game::windows::ecs::{WidgetMessage, ui_action::UiAction},
};

#[derive(Component)]
pub struct Button {
    pub ui_action: UiAction,
    pub hovered: bool,
    pub pressed: bool,
}

impl Button {
    pub fn new(ui_action: UiAction) -> Self {
        Self {
            ui_action,
            hovered: false,
            pressed: false,
        }
    }
}

/// Applies widget pointer messages to buttons and emits their configured UI
/// action when a left-click is completed on the same button.
pub fn update_buttons(
    mut widget_messages: MessageReader<WidgetMessage>,
    mut buttons: Query<&mut Button>,
    mut ui_actions: MessageWriter<UiAction>,
) {
    for message in widget_messages.read() {
        match *message {
            WidgetMessage::Enter(entity) => {
                if let Ok(mut button) = buttons.get_mut(entity) {
                    button.hovered = true;
                }
            }
            WidgetMessage::Exit(entity) => {
                if let Ok(mut button) = buttons.get_mut(entity) {
                    button.hovered = false;
                    button.pressed = false;
                }
            }
            WidgetMessage::MouseDown(entity, MouseButton::Left) => {
                if let Ok(mut button) = buttons.get_mut(entity) {
                    button.pressed = true;
                }
            }
            WidgetMessage::MouseUp(entity, MouseButton::Left) => {
                if let Ok(mut button) = buttons.get_mut(entity) {
                    let was_pressed = button.pressed;
                    button.pressed = false;

                    if was_pressed && button.hovered {
                        ui_actions.write(button.ui_action.clone());
                    }
                }
            }
            WidgetMessage::MouseDown(_, _) | WidgetMessage::MouseUp(_, _) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::{schedule::Schedule, world::World};

    #[test]
    fn update_buttons_emits_action_on_left_click_release() {
        let mut world = World::new();
        world.init_resource::<Messages<WidgetMessage>>();
        world.init_resource::<Messages<UiAction>>();

        let button = world.spawn(Button::new(UiAction::Exit)).id();

        world.write_message(WidgetMessage::Enter(button));
        world.write_message(WidgetMessage::MouseDown(button, MouseButton::Left));
        world.write_message(WidgetMessage::MouseUp(button, MouseButton::Left));

        let mut schedule = Schedule::default();
        schedule.add_systems(update_buttons);
        schedule.run(&mut world);

        let button = world.entity(button).get::<Button>().unwrap();
        assert!(button.hovered);
        assert!(!button.pressed);

        let actions = world
            .resource_mut::<Messages<UiAction>>()
            .drain()
            .collect::<Vec<_>>();
        assert_eq!(actions, vec![UiAction::Exit]);
    }

    #[test]
    fn update_buttons_cancels_click_when_cursor_leaves() {
        let mut world = World::new();
        world.init_resource::<Messages<WidgetMessage>>();
        world.init_resource::<Messages<UiAction>>();

        let button = world.spawn(Button::new(UiAction::Exit)).id();

        world.write_message(WidgetMessage::Enter(button));
        world.write_message(WidgetMessage::MouseDown(button, MouseButton::Left));
        world.write_message(WidgetMessage::Exit(button));
        world.write_message(WidgetMessage::MouseUp(button, MouseButton::Left));

        let mut schedule = Schedule::default();
        schedule.add_systems(update_buttons);
        schedule.run(&mut world);

        let button = world.entity(button).get::<Button>().unwrap();
        assert!(!button.hovered);
        assert!(!button.pressed);
        assert!(world.resource::<Messages<UiAction>>().is_empty());
    }
}
