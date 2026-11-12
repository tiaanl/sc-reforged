use glam::IVec2;

use crate::game::ui::{
    Rect, WidgetResult,
    render::window_renderer::WindowRenderItems,
    windows::{window::WindowRenderContext, window_manager_context::WindowManagerContext},
};

pub trait WidgetBehavior {
    fn on_primary_mouse_down(
        &mut self,
        widget: &mut WidgetCommon,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> WidgetResult {
        let _ = widget;
        let _ = position;
        let _ = context;
        WidgetResult::Ignored
    }

    fn on_secondary_mouse_down(
        &mut self,
        widget: &mut WidgetCommon,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> WidgetResult {
        let _ = widget;
        let _ = position;
        let _ = context;
        WidgetResult::Ignored
    }

    fn on_primary_mouse_up(
        &mut self,
        widget: &mut WidgetCommon,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> WidgetResult {
        let _ = widget;
        let _ = position;
        let _ = context;
        WidgetResult::Ignored
    }

    fn on_secondary_mouse_up(
        &mut self,
        widget: &mut WidgetCommon,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> WidgetResult {
        let _ = widget;
        let _ = position;
        let _ = context;
        WidgetResult::Ignored
    }

    fn on_mouse_wheel(
        &mut self,
        widget: &mut WidgetCommon,
        wheel_steps: i32,
        context: &mut WindowManagerContext,
    ) -> WidgetResult {
        let _ = widget;
        let _ = wheel_steps;
        let _ = context;
        WidgetResult::Ignored
    }

    fn update(&mut self, widget: &mut WidgetCommon, delta_time: f32) {
        let _ = widget;
        let _ = delta_time;
    }

    fn render(
        &mut self,
        widget: &mut WidgetCommon,
        origin: IVec2,
        delta_time_ms: i32,
        window_render_context: &mut WindowRenderContext<'_>,
        window_render_items: &mut WindowRenderItems,
    );
}

pub struct WidgetCommon {
    pub rect: Rect,
    pub color: u32,
    pub parent_index: Option<usize>,
    pub visible: bool,
    pub accepts_input: bool,
    pub hovered: bool,
    pub group_index: Option<usize>,
}

impl Default for WidgetCommon {
    fn default() -> Self {
        Self {
            rect: Rect::default(),
            color: 0x00000000,
            parent_index: None,
            visible: true,
            accepts_input: true,
            hovered: false,
            group_index: None,
        }
    }
}

#[derive(Default)]
pub struct Widget {
    pub common: WidgetCommon,
    pub behavior: Option<Box<dyn WidgetBehavior>>,
}

impl Widget {
    pub fn new(rect: Rect, behavior: Box<dyn WidgetBehavior>) -> Self {
        Self {
            common: WidgetCommon {
                rect,
                ..Default::default()
            },
            behavior: Some(behavior),
        }
    }

    pub fn on_primary_mouse_down(
        &mut self,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> WidgetResult {
        if let Some(ref mut behavior) = self.behavior {
            return behavior.on_primary_mouse_down(&mut self.common, position, context);
        }
        WidgetResult::Ignored
    }

    pub fn on_secondary_mouse_down(
        &mut self,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> WidgetResult {
        if let Some(ref mut behavior) = self.behavior {
            return behavior.on_secondary_mouse_down(&mut self.common, position, context);
        }
        WidgetResult::Ignored
    }

    pub fn on_primary_mouse_up(
        &mut self,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> WidgetResult {
        if let Some(ref mut behavior) = self.behavior {
            return behavior.on_primary_mouse_up(&mut self.common, position, context);
        }
        WidgetResult::Ignored
    }

    pub fn on_secondary_mouse_up(
        &mut self,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> WidgetResult {
        if let Some(ref mut behavior) = self.behavior {
            return behavior.on_secondary_mouse_up(&mut self.common, position, context);
        }
        WidgetResult::Ignored
    }

    pub fn on_mouse_wheel(
        &mut self,
        wheel_steps: i32,
        context: &mut WindowManagerContext,
    ) -> WidgetResult {
        if let Some(ref mut behavior) = self.behavior {
            return behavior.on_mouse_wheel(&mut self.common, wheel_steps, context);
        }
        WidgetResult::Ignored
    }

    pub fn update(&mut self, delta: f32) {
        if let Some(ref mut behavior) = self.behavior {
            behavior.update(&mut self.common, delta);
        }
    }

    pub fn render(
        &mut self,
        origin: IVec2,
        context: &mut WindowRenderContext,
        render_items: &mut WindowRenderItems,
    ) {
        if let Some(ref mut behavior) = self.behavior {
            behavior.render(&mut self.common, origin, 0, context, render_items);
        }
    }
}
