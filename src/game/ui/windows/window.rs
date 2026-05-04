use glam::IVec2;

use crate::game::ui::{
    EventResult, Rect,
    render::window_renderer::{WindowRenderItems, WindowRenderer},
    windows::window_manager_context::WindowManagerContext,
};

pub trait Window {
    /// Return true if the window is modal and should exclusively receive input.
    fn is_modal(&self) -> bool {
        false
    }

    /// Return true if the window should stay above normal windows.
    fn is_always_on_top(&self) -> bool {
        false
    }

    /// Return true if the window is visible.
    fn is_visible(&self) -> bool;

    /// Return true if the window can receive input events.
    fn wants_input(&self) -> bool;

    /// Return true if the global coordinates are within the bounds of the
    /// window.
    fn hit_test(&self, position: IVec2) -> bool;

    /// Return the [Rect] of the window.
    fn rect(&self) -> Rect;

    /// Handle a primary mouse down event.
    fn on_primary_mouse_down(
        &mut self,
        mouse: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let _ = mouse;
        let _ = context;
        EventResult::Ignore
    }

    /// Handle a secondary mouse down event.
    fn on_secondary_mouse_down(
        &mut self,
        mouse: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let _ = mouse;
        let _ = context;
        EventResult::Ignore
    }

    /// Handle a primary mouse up event.
    fn on_primary_mouse_up(
        &mut self,
        mouse: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let _ = mouse;
        let _ = context;
        EventResult::Ignore
    }

    /// Handle a secondary mouse up event.
    fn on_secondary_mouse_up(
        &mut self,
        mouse: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let _ = mouse;
        let _ = context;
        EventResult::Ignore
    }

    /// Handle a mouse wheel event in window-local coordinates.
    fn on_mouse_wheel(
        &mut self,
        mouse: IVec2,
        wheel_steps: i32,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let _ = mouse;
        let _ = wheel_steps;
        let _ = context;
        EventResult::Ignore
    }

    /// Called to update the window state given the time in seconds since the last frame was drawn
    /// in `delta_time`.
    fn update(&mut self, delta_time: f32) {
        let _ = delta_time;
    }

    /// Called for each window so they can append items to the `render_items` to be rendered.
    fn render(&mut self, window_renderer: &WindowRenderer, render_items: &mut WindowRenderItems);
}
