use glam::IVec2;

use crate::game::ui::{
    EventResult, Rect,
    render::window_renderer::{WindowRenderItems, WindowRenderer},
    windows::window_manager_context::WindowManagerContext,
};

pub trait Widget {
    /// Returns the widget rect in its parent window's coordinate space.
    fn rect(&self) -> Rect;

    fn on_primary_mouse_down(&mut self, mouse_position: IVec2) -> EventResult;
    fn on_primary_mouse_up(&mut self, mouse_position: IVec2) -> EventResult;
    fn on_mouse_wheel(&mut self, wheel_steps: i32) -> EventResult;

    fn render(
        &mut self,
        origin: IVec2,
        delta_time_ms: i32,
        window_renderer: &WindowRenderer,
        window_render_items: &mut WindowRenderItems,
    );
}

#[derive(Default)]
pub struct Widgets {
    widgets: Vec<Box<dyn Widget>>,
}

impl Widgets {
    /// Adds a widget to the end of the child-widget list.
    pub fn add(&mut self, widget: Box<dyn Widget>) {
        self.widgets.push(widget);
    }

    /// Forwards a primary mouse-down event to the topmost child widget under
    /// the cursor.
    pub fn on_primary_mouse_down(
        &mut self,
        mouse_position: IVec2,
        _context: &mut WindowManagerContext,
    ) -> EventResult {
        for widget in self.widgets.iter_mut().rev() {
            let rect = widget.rect();
            if !rect.contains(mouse_position) {
                continue;
            }

            let result = widget.on_primary_mouse_down(mouse_position);

            if matches!(result, EventResult::Handled) {
                return result;
            }
        }

        EventResult::Ignore
    }

    /// Forwards a primary mouse-up event to the topmost child widget under
    /// the cursor.
    pub fn on_primary_mouse_up(
        &mut self,
        mouse_position: IVec2,
        _context: &mut WindowManagerContext,
    ) -> EventResult {
        for widget in self.widgets.iter_mut().rev() {
            let rect = widget.rect();
            if !rect.contains(mouse_position) {
                continue;
            }

            let result = widget.on_primary_mouse_up(mouse_position);

            if matches!(result, EventResult::Handled) {
                return result;
            }
        }

        // Give the remaining widgets a chance to clear any pressed state even
        // when the cursor was released outside their bounds.
        for widget in self.widgets.iter_mut().rev() {
            let rect = widget.rect();
            if rect.contains(mouse_position) {
                continue;
            }

            let result = widget.on_primary_mouse_up(mouse_position);

            if matches!(result, EventResult::Handled) {
                return result;
            }
        }

        EventResult::Ignore
    }

    pub fn on_secondary_mouse_down(
        &mut self,
        _position: IVec2,
        _context: &mut WindowManagerContext,
    ) -> EventResult {
        EventResult::Ignore
    }

    pub fn on_secondary_mouse_up(
        &mut self,
        _position: IVec2,
        _context: &mut WindowManagerContext,
    ) -> EventResult {
        EventResult::Ignore
    }

    /// Forwards a mouse-wheel event to the topmost child widget under the
    /// cursor.
    pub fn on_mouse_wheel(
        &mut self,
        mouse_position: IVec2,
        wheel_steps: i32,
        _context: &mut WindowManagerContext,
    ) -> EventResult {
        for widget in self.widgets.iter_mut().rev() {
            let rect = widget.rect();
            if !rect.contains(mouse_position) {
                continue;
            }

            let result = widget.on_mouse_wheel(wheel_steps);

            if matches!(result, EventResult::Handled) {
                return result;
            }
        }

        EventResult::Ignore
    }

    /// Renders child widgets using the original engine's back-to-front widget
    /// traversal order.
    pub fn render(
        &mut self,
        origin: IVec2,
        delta_time_ms: i32,
        window_renderer: &WindowRenderer,
        window_render_items: &mut WindowRenderItems,
    ) {
        for widget in self.widgets.iter_mut().rev() {
            widget.render(origin, delta_time_ms, window_renderer, window_render_items);
        }
    }
}
