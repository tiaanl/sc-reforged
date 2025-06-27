use crate::engine::prelude::*;
use crate::engine::ui;

pub struct UiTestScene {
    ui: ui::Context,
    ui_render_context: ui::RenderContext,
    screen_size: ui::Size,
}

impl UiTestScene {
    pub fn new(renderer: &Renderer, screen_size: ui::Size) -> Self {
        Self {
            ui: ui::Context::new(screen_size),
            ui_render_context: ui::RenderContext::new(renderer),
            screen_size,
        }
    }
}

impl Scene for UiTestScene {
    fn resize(&mut self, renderer: &Renderer) {
        self.screen_size = ui::Size {
            width: renderer.surface_config.width,
            height: renderer.surface_config.height,
        };
    }

    fn render(&mut self, frame: &mut Frame) {
        self.ui.layout(self.screen_size);

        self.ui.render(&mut self.ui_render_context);

        frame._clear_color_and_depth(
            wgpu::Color {
                r: 0.1,
                g: 0.2,
                b: 0.3,
                a: 1.0,
            },
            1.0,
        );

        self.ui_render_context.render(frame);
    }
}
