use glam::IVec2;

use crate::game::ui::{
    EventResult, Rect,
    render::window_renderer::WindowRenderItems,
    windows::{window::WindowRenderContext, window_manager_context::WindowManagerContext},
};

pub trait WidgetImpl {
    fn on_primary_mouse_down(
        &mut self,
        widget: &mut WidgetCommon,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let _ = position;
        let _ = context;
        EventResult::Ignore
    }

    fn on_secondary_mouse_down(
        &mut self,
        widget: &mut WidgetCommon,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let _ = position;
        let _ = context;
        EventResult::Ignore
    }

    fn on_primary_mouse_up(
        &mut self,
        widget: &mut WidgetCommon,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let _ = position;
        let _ = context;
        EventResult::Ignore
    }

    fn on_secondary_mouse_up(
        &mut self,
        widget: &mut WidgetCommon,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let _ = position;
        let _ = context;
        EventResult::Ignore
    }

    fn on_mouse_wheel(
        &mut self,
        widget: &mut WidgetCommon,
        wheel_steps: i32,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let _ = wheel_steps;
        let _ = context;
        EventResult::Ignore
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
    pub thing: Option<Box<dyn WidgetImpl>>,
}

impl Widget {
    pub fn on_primary_mouse_down(
        &mut self,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        if let Some(ref mut thing) = self.thing {
            return thing.on_primary_mouse_down(&mut self.common, position, context);
        }
        EventResult::Ignore
    }

    pub fn on_secondary_mouse_down(
        &mut self,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        if let Some(ref mut thing) = self.thing {
            return thing.on_secondary_mouse_down(&mut self.common, position, context);
        }
        EventResult::Ignore
    }

    pub fn on_primary_mouse_up(
        &mut self,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        if let Some(ref mut thing) = self.thing {
            return thing.on_primary_mouse_up(&mut self.common, position, context);
        }
        EventResult::Ignore
    }

    pub fn on_secondary_mouse_up(
        &mut self,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        if let Some(ref mut thing) = self.thing {
            return thing.on_secondary_mouse_up(&mut self.common, position, context);
        }
        EventResult::Ignore
    }

    pub fn on_mouse_wheel(
        &mut self,
        wheel_steps: i32,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        if let Some(ref mut thing) = self.thing {
            return thing.on_mouse_wheel(&mut self.common, wheel_steps, context);
        }
        EventResult::Ignore
    }

    pub fn render(
        &mut self,
        origin: IVec2,
        context: &mut WindowRenderContext,
        render_items: &mut WindowRenderItems,
    ) {
        if let Some(ref mut thing) = self.thing {
            thing.render(&mut self.common, origin, 0, context, render_items);
        }
    }
}
