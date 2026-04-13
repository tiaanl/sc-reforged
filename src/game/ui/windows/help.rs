use glam::{IVec2, UVec2, Vec4};

use crate::game::{
    config::help_window_defs::HelpDef,
    ui::{
        render::window_renderer::{Font, WindowRenderItems, WindowRenderer},
        widgets::{text_button::TextButtonWidget, widget::Widgets},
    },
};

use super::window::Window;

pub struct HelpWindow {
    pos: IVec2,
    size: UVec2,
    widgets: Widgets,

    should_pause_game: bool,
}

impl HelpWindow {
    pub fn new(help_def: &HelpDef) -> Self {
        let size = help_def.dimensions.unwrap_or(UVec2::new(380, 180));

        let pos = help_def
            .position
            .unwrap_or(IVec2::new(640, 480) / 2 - size.as_ivec2() / 2);

        let mut widgets = Widgets::default();

        if help_def.is_confirmation {
            let button_width = (size.x as i32 - 64) / 3;

            let mut button = Box::new(TextButtonWidget::new(
                IVec2::new(button_width + 32, size.y as i32 - 36),
                UVec2::new(button_width as u32, 20),
                help_def.confirmation_text_1.as_ref().unwrap(),
            ));
            button.font = Font::TwelvePoint;
            button.custom_color = Some(Vec4::new(25.0 / 255.0, 1.0, 25.0 / 255.0, 1.0)); // 0xff19ff19

            widgets.add(button);

            let mut button = Box::new(TextButtonWidget::new(
                IVec2::new(button_width * 2 + 48, size.y as i32 - 36),
                UVec2::new(button_width as u32, 20),
                help_def.confirmation_text_2.as_ref().unwrap(),
            ));
            button.font = Font::TwelvePoint;
            button.custom_color = Some(Vec4::new(25.0 / 255.0, 1.0, 25.0 / 255.0, 1.0)); // 0xff19ff19

            widgets.add(button);
        }

        Self {
            pos,
            size,
            widgets,
            should_pause_game: !help_def.do_not_pause_game,
        }
    }
}

impl Window for HelpWindow {
    fn render(&mut self, window_renderer: &WindowRenderer, render_items: &mut WindowRenderItems) {
        if self.should_pause_game {
            // Render modal background.
            // Render_Solid_Rect(0,0,g_renderer->m_screen_width,g_renderer->m_screen_height,0x50000000);
            render_items.render_solid_rect(
                IVec2::ZERO,
                UVec2::new(640, 480),
                Vec4::new(0.0, 0.0, 0.0, 80.0 / 255.0),
            );

            // Render an inner dark rect.
            // Render_Solid_Rect(left + 16,top + 16,width + -32,height + -68,0x80000000);
            render_items.render_solid_rect(
                self.pos + IVec2::splat(16),
                self.size - UVec2::new(32, 68),
                Vec4::new(0.0, 0.0, 0.0, 128.0 / 255.0),
            );
        }

        // Render a border around the contents.
        // Render_Single_Pixel_Border(left + -1,top + -1,width + 1 + left,top + 1 + height,0xff19ff19);
        render_items.render_border(
            self.pos - IVec2::ONE,
            self.size + UVec2::splat(2),
            1,
            Vec4::new(25.0 / 255.0, 1.0, 25.0 / 255.0, 1.0),
        );

        // Render the background for the window.
        // Render_Solid_Rect(left,top,width,height,0x50000000);
        render_items.render_solid_rect(self.pos, self.size, Vec4::new(0.0, 0.0, 0.0, 80.0 / 255.0));

        render_items.render_text(
            glam::IVec2::new(20, 20),
            "Hello, World!",
            Font::FifteenPoint,
            None,
        );

        // TODO: Render pointer.

        self.widgets.render(self.pos, window_renderer, render_items);
    }
}
