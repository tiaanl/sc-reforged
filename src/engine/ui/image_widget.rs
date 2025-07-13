use crate::engine::ui::*;

use super::Size;

#[derive(Default)]
pub struct ImageWidget {
    rect: Rect,

    pub style: Style,
}

impl Widget for ImageWidget {
    fn style(&self) -> &Style {
        &self.style
    }

    fn min_size(&self) -> Size {
        Size::ZERO
    }

    fn layout(&mut self, constraint: Rect, context: &layout::LayoutContext) {
        self.rect = constraint;
    }

    fn render(&self, render_context: &mut RenderContext) {
        render_context.render_color(self.rect, self.style.background_color.into());
    }
}
