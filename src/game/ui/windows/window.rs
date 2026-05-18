use glam::IVec2;

use crate::{
    engine::renderer::RenderContext,
    game::ui::{
        EventResult, Rect,
        render::window_renderer::{WindowRenderItems, WindowRenderer},
        widgets::widget::Widgets,
        windows::window_manager_context::WindowManagerContext,
    },
};

/// Resources passed to each window during the render phase. Carries enough to
/// drive direct GPU work (e.g. world rendering into a gbuffer) in addition to
/// emitting window render items.
pub struct WindowRenderContext<'a> {
    pub render_context: &'a mut RenderContext,
    pub window_renderer: &'a WindowRenderer,
}

pub struct WindowCommon {
    pub rect: Rect,
    // geometries: Geometries,
    pub widgets: Widgets,

    pub is_visible: bool,
    pub is_enabled: bool,
    pub is_modal: bool,
    pub is_always_on_top: bool,
}

impl WindowCommon {
    pub fn new(rect: Rect) -> Self {
        Self {
            rect,
            widgets: Widgets::default(),

            is_visible: true,
            is_enabled: true,
            is_modal: false,
            is_always_on_top: false,
        }
    }
}

pub trait WindowImpl {
    /// Handle a primary mouse down event.
    fn on_primary_mouse_down(
        &mut self,
        common: &mut WindowCommon,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let _ = common;
        let _ = position;
        let _ = context;
        EventResult::Ignore
    }

    /// Handle a secondary mouse down event.
    fn on_secondary_mouse_down(
        &mut self,
        common: &mut WindowCommon,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let _ = common;
        let _ = position;
        let _ = context;
        EventResult::Ignore
    }

    /// Handle a primary mouse up event.
    fn on_primary_mouse_up(
        &mut self,
        common: &mut WindowCommon,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let _ = common;
        let _ = position;
        let _ = context;
        EventResult::Ignore
    }

    /// Handle a secondary mouse up event.
    fn on_secondary_mouse_up(
        &mut self,
        common: &mut WindowCommon,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let _ = common;
        let _ = position;
        let _ = context;
        EventResult::Ignore
    }

    /// Handle a mouse wheel event in window-local coordinates.
    fn on_mouse_wheel(
        &mut self,
        common: &mut WindowCommon,
        position: IVec2,
        wheel_steps: i32,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let _ = common;
        let _ = position;
        let _ = wheel_steps;
        let _ = context;
        EventResult::Ignore
    }

    /// Called to update the window state given the time in seconds since the last frame was drawn
    /// in `delta_time`.
    fn update(&mut self, delta_time: f32) {
        let _ = delta_time;
    }

    /// Called when the logical UI size changes so windows can re-resolve any
    /// expression-based layout (button positions, geometry, etc.) keyed off
    /// `%screen_dx` / `%screen_dy`.
    fn on_resize(&mut self, common: &mut WindowCommon, logical_size: IVec2) {
        let _ = common;
        let _ = logical_size;
    }

    /// Called for each window so they can drive any GPU work and append items
    /// to `render_items` to be composited later.
    ///
    /// The default implementation renders `common.widgets`. Overrides are
    /// responsible for calling `common.widgets.render(...)` themselves if they
    /// want child widgets to draw.
    fn render(
        &mut self,
        common: &mut WindowCommon,
        context: &mut WindowRenderContext<'_>,
        render_items: &mut WindowRenderItems,
    ) {
        common
            .widgets
            .render(common.rect.position, 0, context, render_items);
    }
}

pub struct Window {
    common: WindowCommon,
    window_impl: Box<dyn WindowImpl>,
}

impl Window {
    pub fn new(common: WindowCommon, window_impl: Box<dyn WindowImpl>) -> Self {
        Self {
            common,
            window_impl,
        }
    }

    /// Return true if the window is visible.
    pub fn is_visible(&self) -> bool {
        self.common.is_visible
    }

    /// Return true if the window can receive input events.
    pub fn is_enabled(&self) -> bool {
        self.common.is_enabled
    }

    /// Return true if the window is modal and should exclusively receive input.
    pub fn is_modal(&self) -> bool {
        self.common.is_modal
    }

    /// Return true if the window should stay above normal windows.
    pub fn is_always_on_top(&self) -> bool {
        self.common.is_always_on_top
    }

    /// Return true if the global coordinates are within the bounds of the
    /// window.
    pub fn hit_test(&self, position: IVec2) -> bool {
        self.common.rect.contains(position)
    }

    /// Return the [Rect] of the window.
    pub fn rect(&self) -> Rect {
        self.common.rect
    }

    pub fn on_resize(&mut self, logical_size: IVec2) {
        self.window_impl.on_resize(&mut self.common, logical_size);
    }

    pub fn on_primary_mouse_down(
        &mut self,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let result = self.common.widgets.on_primary_mouse_down(position, context);
        if matches!(result, EventResult::Handled) {
            return result;
        }
        self.window_impl
            .on_primary_mouse_down(&mut self.common, position, context)
    }

    pub fn on_primary_mouse_up(
        &mut self,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let result = self.common.widgets.on_primary_mouse_up(position, context);
        if matches!(result, EventResult::Handled) {
            return result;
        }
        self.window_impl
            .on_primary_mouse_up(&mut self.common, position, context)
    }

    pub fn on_secondary_mouse_down(
        &mut self,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let result = self.common.widgets.on_secondary_mouse_down(position, context);
        if matches!(result, EventResult::Handled) {
            return result;
        }
        self.window_impl
            .on_secondary_mouse_down(&mut self.common, position, context)
    }

    pub fn on_secondary_mouse_up(
        &mut self,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let result = self.common.widgets.on_secondary_mouse_up(position, context);
        if matches!(result, EventResult::Handled) {
            return result;
        }
        self.window_impl
            .on_secondary_mouse_up(&mut self.common, position, context)
    }

    pub fn on_mouse_wheel(
        &mut self,
        position: IVec2,
        wheel_steps: i32,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let result = self
            .common
            .widgets
            .on_mouse_wheel(position, wheel_steps, context);
        if matches!(result, EventResult::Handled) {
            return result;
        }
        self.window_impl
            .on_mouse_wheel(&mut self.common, position, wheel_steps, context)
    }

    pub fn render(
        &mut self,
        context: &mut WindowRenderContext<'_>,
        render_items: &mut WindowRenderItems,
    ) {
        self.window_impl
            .render(&mut self.common, context, render_items);
    }
}
