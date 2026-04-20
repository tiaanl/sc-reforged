use glam::IVec2;

use crate::game::ui::{
    EventResult,
    render::window_renderer::{WindowRenderItems, WindowRenderer},
};

pub trait Widget {
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
    pub fn add(&mut self, widget: Box<dyn Widget>) {
        self.widgets.push(widget);
    }

    pub fn render(
        &mut self,
        origin: IVec2,
        delta_time_ms: i32,
        window_renderer: &WindowRenderer,
        window_render_items: &mut WindowRenderItems,
    ) {
        for widget in self.widgets.iter_mut() {
            widget.render(origin, delta_time_ms, window_renderer, window_render_items);
        }
    }
}
