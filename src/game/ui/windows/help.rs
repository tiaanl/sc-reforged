use glam::{IVec2, Vec4};

use crate::game::{
    config::help_window_defs::HelpDef,
    ui::{
        Rect,
        render::window_renderer::{Font, WindowRenderItems, WindowRenderer},
        u32_to_color,
        widgets::{
            list_box::{ListBoxItem, ListBoxWidget},
            text_button::TextButtonWidget,
            widget::Widgets,
        },
    },
};

use super::window::Window;

pub struct HelpWindow {
    rect: Rect,
    widgets: Widgets,

    should_pause_game: bool,
}

impl HelpWindow {
    pub fn new(help_def: &HelpDef) -> Self {
        let size = help_def.dimensions.unwrap_or(IVec2::new(380, 180));

        // TODO: Center against the current render surface size instead of the
        // hardcoded 640x480 fallback so the help window follows the original
        // positioning logic at whatever resolution the renderer is using.
        let pos = help_def
            .position
            .unwrap_or(IVec2::new(640, 480) / 2 - size / 2);

        let mut widgets = Widgets::default();

        // List box widget to hold the help text body lines.
        let mut list_box = Box::new(ListBoxWidget::vertical(Rect::new(
            IVec2::splat(16),
            size - IVec2::new(32, 68),
        )));
        // TODO: Match the original help-body list-box configuration here:
        // it should be a plain clipped container for body rows, with no panel
        // background and no extra border beyond the surrounding help window.

        for line in help_def.body_lines.iter() {
            let list_item = ListBoxItem::text(
                line.clone(),
                Font::TwelvePoint,
                Some(u32_to_color(0xff19ff19)),
            );
            list_box.add_item(list_item);
        }

        widgets.add(list_box);

        if help_def.is_confirmation {
            let button_width = (size.x - 64) / 3;
            // TODO: The original confirmation buttons are not just visual.
            // Wire them up to the help-def action/callback data so Quit and
            // Cancel actually trigger the expected behavior.

            let mut button = Box::new(TextButtonWidget::new(
                Rect::new(
                    IVec2::new(button_width + 32, size.y - 36),
                    IVec2::new(button_width, 20),
                ),
                help_def.confirmation_text_1.as_ref().unwrap(),
            ));
            button.font = Font::TwelvePoint;
            button.custom_color = Some(Vec4::new(25.0 / 255.0, 1.0, 25.0 / 255.0, 1.0)); // 0xff19ff19

            widgets.add(button);

            let mut button = Box::new(TextButtonWidget::new(
                Rect::new(
                    IVec2::new(button_width * 2 + 48, size.y - 36),
                    IVec2::new(button_width, 20),
                ),
                help_def.confirmation_text_2.as_ref().unwrap(),
            ));
            button.font = Font::TwelvePoint;
            button.custom_color = Some(Vec4::new(25.0 / 255.0, 1.0, 25.0 / 255.0, 1.0)); // 0xff19ff19

            widgets.add(button);
        }

        Self {
            rect: Rect::new(pos, size),
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
            // TODO: Use the current render surface dimensions here instead of
            // the fixed 640x480 backdrop.
            render_items.render_solid_rect(
                Rect::from_size(IVec2::new(640, 480)),
                Vec4::new(0.0, 0.0, 0.0, 80.0 / 255.0),
            );

            // Render an inner dark rect.
            // Render_Solid_Rect(left + 16,top + 16,width + -32,height + -68,0x80000000);
            render_items.render_solid_rect(
                self.rect.offset(IVec2::splat(16)).grow(-IVec2::new(32, 68)),
                Vec4::new(0.0, 0.0, 0.0, 128.0 / 255.0),
            );
        }

        // Render a border around the contents.
        // Render_Single_Pixel_Border(left + -1,top + -1,width + 1 + left,top + 1 + height,0xff19ff19);
        render_items.render_border(
            self.rect.offset(-IVec2::ONE).grow(IVec2::splat(2)),
            1,
            Vec4::new(25.0 / 255.0, 1.0, 25.0 / 255.0, 1.0),
        );

        // Render the background for the window.
        // Render_Solid_Rect(left,top,width,height,0x50000000);
        render_items.render_solid_rect(self.rect, Vec4::new(0.0, 0.0, 0.0, 80.0 / 255.0));

        // TODO: Replace this placeholder with the actual help-window title and
        // title placement from the original help window. The quit-confirmation
        // dialog should not keep this temporary text once the body list and
        // title rendering are in place.
        render_items.render_text(
            glam::IVec2::new(20, 20),
            b"Hello, World!",
            Font::FifteenPoint,
            None,
        );

        // TODO: Render the help pointer only for defs that specify one. The
        // quit-confirmation dialog likely does not need this, but other help
        // windows do.

        const DELTA_TIME_MS: i32 = 10;

        self.widgets.render(
            self.rect.position,
            DELTA_TIME_MS,
            window_renderer,
            render_items,
        );
    }
}
