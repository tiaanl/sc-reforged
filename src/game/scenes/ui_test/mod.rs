use crate::engine::prelude::*;
use crate::engine::ui;
use crate::engine::ui::Color;
use crate::engine::ui::ImageWidget;
use crate::engine::ui::Length;
use crate::engine::ui::PanelWidget;
use crate::engine::ui::Style;
use crate::engine::ui::UI_PIXEL_SCALE;
use crate::engine::ui::WidgetContainerExt;

pub struct UiTestScene {
    ui: ui::Context,
    ui_render_context: ui::RenderContext,
    screen_size: ui::Size,
}

impl UiTestScene {
    pub fn new(renderer: &Renderer, screen_size: ui::Size) -> Self {
        let mut ui = ui::Context::new(screen_size);

        ui.add_to_root(Box::new(PanelWidget::default().with_style(Style {
            width: Length::Pixels(100),
            height: Length::Pixels(200),
            background_color: Color::BLUE,
            ..Default::default()
        })));

        ui.add_to_root(Box::new({
            let mut panel = PanelWidget::default().with_style(Style {
                width: Length::Pixels(200),
                height: Length::Pixels(200),
                background_color: Color::RED,
                ..Default::default()
            });
            panel.add_child(Box::new(ImageWidget::default()));
            panel.style.background_color = Color::from_rgba(10, 50, 90, 255);
            panel
        }));

        Self {
            ui,
            ui_render_context: ui::RenderContext::new(renderer, UI_PIXEL_SCALE),
            screen_size,
        }
    }
}

impl Scene for UiTestScene {
    fn resize(&mut self, renderer: &Renderer) {
        self.screen_size = ui::Size {
            width: renderer.surface_config.width as i32,
            height: renderer.surface_config.height as i32,
        };

        self.ui_render_context.resize(self.screen_size);
    }

    fn render(&mut self, frame: &mut Frame) {
        self.ui.layout(self.screen_size);

        self.ui.render(&mut self.ui_render_context);

        // frame._clear_color_and_depth(
        //     wgpu::Color {
        //         r: 0.1,
        //         g: 0.2,
        //         b: 0.3,
        //         a: 1.0,
        //     },
        //     1.0,
        // );

        self.ui_render_context.render(frame);
    }
}
