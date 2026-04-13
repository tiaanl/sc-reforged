use glam::IVec2;

use crate::game::ui::render::window_renderer::{WindowRenderItems, WindowRenderer};

pub trait Widget {
    fn render(
        &self,
        origin: IVec2,
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
        &self,
        origin: IVec2,
        window_renderer: &WindowRenderer,
        window_render_items: &mut WindowRenderItems,
    ) {
        for widget in self.widgets.iter() {
            widget.render(origin, window_renderer, window_render_items);
        }
    }
}
