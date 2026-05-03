use std::collections::HashSet;

use bevy_ecs::prelude::*;
use glam::{IVec2, UVec2};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseScrollDelta, WindowEvent},
    keyboard::PhysicalKey,
};

pub use winit::event::MouseButton;
pub use winit::keyboard::KeyCode;

/// A high-level input event derived from winit window events.
#[derive(Clone, Debug)]
pub enum InputEvent {
    MouseMove(UVec2),
    MouseDown(MouseButton),
    MouseWheel(f32),
    MouseUp(MouseButton),
    MouseLeave,
    KeyDown(KeyCode),
    KeyUp(KeyCode),
}

/// Converts a winit `WindowEvent` into an `InputEvent`, if applicable.
pub fn translate_window_event(event: &WindowEvent) -> Option<InputEvent> {
    match event {
        WindowEvent::KeyboardInput { event, .. } if !event.repeat => {
            let PhysicalKey::Code(key) = event.physical_key else {
                return None;
            };
            if event.state == ElementState::Pressed {
                Some(InputEvent::KeyDown(key))
            } else {
                Some(InputEvent::KeyUp(key))
            }
        }

        WindowEvent::CursorMoved {
            position: PhysicalPosition { x, y },
            ..
        } => Some(InputEvent::MouseMove(UVec2::new(
            x.round() as u32,
            y.round() as u32,
        ))),

        WindowEvent::CursorLeft { .. } => Some(InputEvent::MouseLeave),

        WindowEvent::MouseWheel { delta, .. } => {
            let delta = match delta {
                MouseScrollDelta::LineDelta(_, y) => *y,
                MouseScrollDelta::PixelDelta(PhysicalPosition { y, .. }) => *y as f32,
            };
            Some(InputEvent::MouseWheel(delta))
        }

        WindowEvent::MouseInput { state, button, .. } => {
            if state.is_pressed() {
                Some(InputEvent::MouseDown(*button))
            } else {
                Some(InputEvent::MouseUp(*button))
            }
        }

        _ => None,
    }
}

#[derive(Clone, Default, Resource)]
pub struct InputState {
    /// The current position of the mouse inside the window client area in pixels. Set to `None` If
    /// the mouse is not over the client area.
    mouse_position: Option<UVec2>,
    last_mouse_position: Option<UVec2>,
    mouse_delta: Option<IVec2>,

    mouse_pressed: HashSet<MouseButton>,
    mouse_just_pressed: HashSet<MouseButton>,
    mouse_just_released: HashSet<MouseButton>,

    key_pressed: HashSet<KeyCode>,
    key_just_pressed: HashSet<KeyCode>,

    wheel_delta: f32,
}

impl InputState {
    /// Apply a single input event, updating the accumulated state.
    pub fn apply(&mut self, event: &InputEvent) {
        match *event {
            InputEvent::KeyDown(key) => {
                self.key_pressed.insert(key);
                self.key_just_pressed.insert(key);
            }
            InputEvent::KeyUp(key) => {
                self.key_pressed.remove(&key);
            }
            InputEvent::MouseMove(position) => {
                self.last_mouse_position = self.mouse_position;
                if let Some(last) = self.last_mouse_position {
                    self.mouse_delta = Some(last.as_ivec2() - position.as_ivec2());
                }
                self.mouse_position = Some(position);
            }
            InputEvent::MouseLeave => {
                self.mouse_position = None;
            }
            InputEvent::MouseWheel(delta) => {
                self.wheel_delta = delta;
            }
            InputEvent::MouseDown(button) => {
                self.mouse_pressed.insert(button);
                self.mouse_just_pressed.insert(button);
            }
            InputEvent::MouseUp(button) => {
                self.mouse_pressed.remove(&button);
                self.mouse_just_released.insert(button);
            }
        }
    }

    /// Reset per-frame transient state (e.g. "just pressed", "just released", deltas).
    /// Call once per frame after all systems have read the input.
    pub fn reset_per_frame(&mut self) {
        self.key_just_pressed.clear();
        self.mouse_just_pressed.clear();
        self.mouse_just_released.clear();
        self.mouse_delta = None;
        self.wheel_delta = 0.0;
    }
}

impl InputState {
    pub fn mouse_position(&self) -> Option<UVec2> {
        self.mouse_position
    }

    pub fn key_pressed(&self, key: KeyCode) -> bool {
        self.key_pressed.contains(&key)
    }

    pub fn key_just_pressed(&self, key: KeyCode) -> bool {
        self.key_just_pressed.contains(&key)
    }

    pub fn mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse_pressed.contains(&button)
    }

    pub fn mouse_just_pressed(&self, button: MouseButton) -> bool {
        self.mouse_just_pressed.contains(&button)
    }

    pub fn mouse_just_released(&self, button: MouseButton) -> bool {
        self.mouse_just_released.contains(&button)
    }

    pub fn mouse_delta(&self) -> Option<IVec2> {
        self.mouse_delta
    }

    pub fn wheel_delta(&self) -> f32 {
        self.wheel_delta
    }
}
