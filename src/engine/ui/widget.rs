use crate::engine::ui::{Rect, RenderContext, Size, layout::LayoutContext};

pub trait Widget {
    fn min_size(&self) -> Size;
    fn layout(&mut self, constraint: Rect, context: &LayoutContext);
    fn render(&self, render_context: &mut RenderContext);
}

pub type DynWidget = Box<dyn Widget>;

pub trait WidgetContainerExt {
    fn add_child(&mut self, child: DynWidget);
}

#[derive(Default)]
pub struct WidgetContainer {
    children: Vec<DynWidget>,
}

impl WidgetContainer {
    pub fn iter(&self) -> impl Iterator<Item = &DynWidget> {
        self.children.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut DynWidget> {
        self.children.iter_mut()
    }
}

impl WidgetContainerExt for WidgetContainer {
    fn add_child(&mut self, child: DynWidget) {
        self.children.push(child)
    }
}
