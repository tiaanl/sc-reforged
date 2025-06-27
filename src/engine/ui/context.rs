use crate::engine::ui::{
    RenderContext,
    geometry::{Pos, Rect, Size},
    layout::LayoutContext,
    root_widget::RootWidget,
    widget::Widget,
};

pub struct Context {
    root_widget: RootWidget,
}

impl Context {
    pub fn new(screen_size: Size) -> Self {
        let root_widget = RootWidget::new(screen_size);

        Self { root_widget }
    }

    pub fn layout(&mut self, screen_size: Size) {
        let layout_context = LayoutContext { screen_size };
        let parent_rect = Rect {
            pos: Pos::ZERO,
            size: screen_size,
        };
        self.root_widget.layout(parent_rect, &layout_context);
    }

    pub fn render(&self, render_context: &mut RenderContext) {
        self.root_widget.render(render_context);
    }
}
