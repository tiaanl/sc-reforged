use glam::IVec2;

use crate::{
    engine::input::InputState,
    game::scenes::world::sim_world::{SelectionRect, SimWorld},
};

pub struct UiSystem;

impl UiSystem {
    pub fn input(&mut self, sim_world: &mut SimWorld, input_state: &InputState) {
        if input_state.mouse_just_pressed(winit::event::MouseButton::Left) {
            sim_world.ui.selection_rect = input_state.mouse_position().map(|pos| SelectionRect {
                pos,
                size: IVec2::ZERO,
            });
            return;
        }

        if input_state.mouse_just_released(winit::event::MouseButton::Left) {
            sim_world.ui.selection_rect = None;
            return;
        }

        let Some(rect) = sim_world.ui.selection_rect.as_mut() else {
            return;
        };

        if let Some(mouse_position) = input_state.mouse_position() {
            rect.size = mouse_position - rect.pos;
        }
    }
}
