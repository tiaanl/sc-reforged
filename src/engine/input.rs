use std::collections::HashSet;

use glam::{IVec2, UVec2};
use winit::{
    event::{ElementState, MouseScrollDelta, WindowEvent},
    keyboard::PhysicalKey,
};

pub use winit::event::MouseButton;
pub use winit::keyboard::KeyCode;

#[derive(Default)]
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
    pub(crate) fn handle_window_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { ref event, .. } if !event.repeat => {
                if let PhysicalKey::Code(key) = event.physical_key {
                    if event.state == ElementState::Pressed {
                        self.key_pressed.insert(key);
                        self.key_just_pressed.insert(key);
                    } else {
                        self.key_pressed.remove(&key);
                    }
                }
            }

            WindowEvent::CursorMoved {
                position: winit::dpi::PhysicalPosition { x, y },
                ..
            } => {
                self.last_mouse_position = self.mouse_position;
                let current = UVec2::new(x.round() as u32, y.round() as u32);

                if let Some(last) = self.last_mouse_position {
                    self.mouse_delta = Some(last.as_ivec2() - current.as_ivec2());
                }

                self.mouse_position = Some(current);
            }

            WindowEvent::CursorLeft { .. } => self.mouse_position = None,

            WindowEvent::MouseWheel { delta, .. } => {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition { y, .. }) => {
                        y as f32
                    }
                };
                self.wheel_delta = delta;
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if state.is_pressed() {
                    self.mouse_pressed.insert(button);
                    self.mouse_just_pressed.insert(button);
                } else {
                    self.mouse_pressed.remove(&button);
                    self.mouse_just_released.insert(button);
                }
            }

            _ => {}
        }
    }

    /// Reset data being tracked per frame.
    pub(crate) fn reset_current_frame(&mut self) {
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
