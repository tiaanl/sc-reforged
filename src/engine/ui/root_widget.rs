use crate::engine::ui::{Pos, Rect, geometry::Size, layout::LayoutContext, widget::Widget};

pub struct RootWidget {
    size: Size,
}

impl RootWidget {
    pub fn new(size: Size) -> Self {
        Self { size }
    }
}

impl Widget for RootWidget {
    fn layout(&mut self, constaint: Rect, _context: &LayoutContext) {
        assert!(constaint.pos == Pos::ZERO);
        self.size = constaint.size;
    }

    fn render(&self, render_context: &mut super::RenderContext) {
        render_context.render_color(Rect {
            pos: Pos::ZERO,
            size: self.size,
        });
    }
}
