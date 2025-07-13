use glam::Vec4;

use crate::engine::ui::{
    Pos, Rect, Style,
    geometry::Size,
    layout::{LayoutContext, layout_horizontal, layout_stacked},
    widget::{DynWidget, Widget, WidgetContainer, WidgetContainerExt},
};

pub struct RootWidget {
    size: Size,
    pub style: Style,

    children: WidgetContainer,
}

impl RootWidget {
    pub fn new(size: Size) -> Self {
        Self {
            size,
            style: Style::default(),
            children: WidgetContainer::default(),
        }
    }
}

impl Widget for RootWidget {
    fn style(&self) -> &Style {
        &self.style
    }

    fn min_size(&self) -> Size {
        self.size
    }

    fn layout(&mut self, constraint: Rect, context: &LayoutContext) {
        self.size = constraint.size;

        layout_stacked(self.children.iter_mut(), constraint, context);
        layout_horizontal(self.children.iter_mut(), constraint, context);
    }

    fn render(&self, render_context: &mut super::RenderContext) {
        render_context.render_color(
            Rect {
                pos: Pos::ZERO,
                size: self.size,
            },
            self.style.background_color.into(),
        );

        for child in self.children.iter() {
            child.render(render_context);
        }
    }
}

impl WidgetContainerExt for RootWidget {
    fn add_child(&mut self, child: DynWidget) {
        self.children.add_child(child)
    }
}
