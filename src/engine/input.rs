use std::collections::HashSet;

use glam::Vec2;
use winit::{
    event::{ElementState, MouseScrollDelta, WindowEvent},
    keyboard::PhysicalKey,
};

pub use winit::event::MouseButton;
pub use winit::keyboard::KeyCode;

#[derive(Default)]
pub struct InputState {
    key_pressed: HashSet<KeyCode>,
    mouse_pressed: HashSet<MouseButton>,
    last_mouse_position: Option<Vec2>,
    mouse_delta: Option<Vec2>,
    wheel_delta: f32,
}

impl InputState {
    pub(crate) fn handle_window_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { ref event, .. } => {
                if let PhysicalKey::Code(key) = event.physical_key {
                    if !event.repeat {
                        if event.state == ElementState::Pressed {
                            self.key_pressed.insert(key);
                        } else {
                            self.key_pressed.remove(&key);
                        }
                    }
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if state.is_pressed() {
                    self.mouse_pressed.insert(button);
                } else {
                    self.mouse_pressed.remove(&button);
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let current = Vec2::new(position.x as f32, position.y as f32);
                self.mouse_delta = self.last_mouse_position.map(|p| current - p);
                self.last_mouse_position = Some(current);
            }

            WindowEvent::MouseWheel { delta, .. } => {
                self.wheel_delta = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(physical_position) => physical_position.y as f32,
                };
            }

            _ => {}
        }
    }

    /// Reset data being tracked per frame.
    pub(crate) fn reset_current_frame(&mut self) {
        self.mouse_delta = None;
        self.wheel_delta = 0.0;
    }
}

impl InputState {
    pub fn key_pressed(&self, key: KeyCode) -> bool {
        self.key_pressed.contains(&key)
    }

    pub fn mouse_position(&self) -> Option<Vec2> {
        self.last_mouse_position
    }

    pub fn mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse_pressed.contains(&button)
    }

    pub fn mouse_delta(&self) -> Option<Vec2> {
        self.mouse_delta
    }

    pub fn wheel_delta(&self) -> f32 {
        self.wheel_delta
    }
}
