use crate::engine::ui::{RenderContext, geometry::Rect, layout::LayoutContext};

pub trait Widget {
    fn layout(&mut self, constraint: Rect, context: &LayoutContext);
    fn render(&self, render_context: &mut RenderContext);
}
