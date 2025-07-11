use crate::engine::ui::{
    Color, RenderContext, UI_PIXEL_SCALE,
    geometry::{Pos, Rect, Size},
    layout::LayoutContext,
    root_widget::RootWidget,
    widget::{DynWidget, Widget, WidgetContainerExt},
};

pub struct Context {
    root_widget: RootWidget,
}

impl Context {
    pub fn new(screen_size: Size) -> Self {
        let mut root_widget = RootWidget::new(screen_size);
        root_widget.style.background_color = Color::from_rgba(40, 40, 40, 255);

        Self { root_widget }
    }

    pub fn add_to_root(&mut self, child: DynWidget) {
        self.root_widget.add_child(child);
    }

    pub fn layout(&mut self, screen_size: Size) {
        let layout_context = LayoutContext {
            screen_size: screen_size * UI_PIXEL_SCALE,
        };
        self.root_widget.layout(
            Rect {
                pos: Pos::ZERO,
                size: layout_context.screen_size,
            },
            &layout_context,
        );
    }

    pub fn render(&self, render_context: &mut RenderContext) {
        self.root_widget.render(render_context);
    }
}
