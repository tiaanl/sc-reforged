use crate::game::ui::render::window_renderer::WindowRenderItems;

pub trait Widget {
    fn render(&mut self, window_render_items: &mut WindowRenderItems);
}

#[derive(Default)]
pub struct Widgets {
    widgets: Vec<Box<dyn Widget>>,
}

impl Widgets {
    pub fn add(&mut self, widget: Box<dyn Widget>) {
        self.widgets.push(widget);
    }

    pub fn render(&self, _window_render_items: &mut WindowRenderItems) {
        //
    }
}
