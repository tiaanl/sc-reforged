use crate::engine::ui::{Rect, widget::Widget};

pub struct PanelWidget {
    rect: Rect,
}

impl Widget for PanelWidget {
    fn layout(&mut self, constaint: Rect, _context: &super::layout::LayoutContext) {
        self.rect = constaint;
    }

    fn render(&self, render_context: &mut super::RenderContext) {
        render_context.render_color(self.rect);
    }
}
