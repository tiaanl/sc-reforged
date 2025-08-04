use crate::engine::ui::*;

use super::{RenderContext, layout::LayoutContext};

#[derive(Default)]
pub struct PanelWidget {
    rect: Rect,
    pub style: Style,
    pub children: WidgetContainer,
}

impl PanelWidget {
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Widget for PanelWidget {
    fn style(&self) -> &Style {
        &self.style
    }

    fn min_size(&self) -> Size {
        Size {
            width: match self.style.width {
                Length::Auto => 0,
                Length::Pixels(pixels) => pixels * UI_PIXEL_SCALE,
            },
            height: match self.style.height {
                Length::Auto => 0,
                Length::Pixels(pixels) => pixels * UI_PIXEL_SCALE,
            },
        }
    }

    fn layout(&mut self, constraint: Rect, _context: &LayoutContext) {
        self.rect = constraint;
    }

    fn render(&self, render_context: &mut RenderContext) {
        render_context.render_color(self.rect, self.style.background_color.into());
    }
}

impl WidgetContainerExt for PanelWidget {
    fn add_child(&mut self, child: DynWidget) {
        self.children.add_child(child)
    }
}
