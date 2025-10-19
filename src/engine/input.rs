use std::collections::HashSet;

use glam::IVec2;
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
    mouse_position: Option<IVec2>,
    last_mouse_position: Option<IVec2>,
    mouse_delta: Option<IVec2>,

    mouse_pressed: HashSet<MouseButton>,
    mouse_just_pressed: HashSet<MouseButton>,

    key_pressed: HashSet<KeyCode>,
    key_just_pressed: HashSet<KeyCode>,

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
                            self.key_just_pressed.insert(key);
                        } else {
                            self.key_pressed.remove(&key);
                        }
                    }
                }
            }

            WindowEvent::CursorMoved {
                position: winit::dpi::PhysicalPosition { x, y },
                ..
            } => {
                self.last_mouse_position = self.mouse_position;
                let current = IVec2::new(x as i32, y as i32);

                if let Some(last) = self.last_mouse_position {
                    self.mouse_delta = Some(last - current);
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
                }
            }

            _ => {}
        }
    }

    /// Reset data being tracked per frame.
    pub(crate) fn reset_current_frame(&mut self) {
        self.key_just_pressed.clear();
        self.mouse_just_pressed.clear();
        self.mouse_delta = None;
        self.wheel_delta = 0.0;
    }
}

impl InputState {
    pub fn _mouse_position(&self) -> Option<IVec2> {
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

    pub fn mouse_delta(&self) -> Option<IVec2> {
        self.mouse_delta
    }

    pub fn _wheel_delta(&self) -> f32 {
        self.wheel_delta
    }
}
