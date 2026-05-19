use glam::{IVec2, UVec2};

use crate::game::{
    config::help_window_defs::HelpDef,
    ui::{
        Rect,
        render::window_renderer::{Font, WindowRenderItems},
        u32_to_color,
        widgets::{
            list_box::{ListBoxItem, ListBoxWidget},
            text_button::TextButtonWidget,
        },
        windows::window::{Window, WindowCommon},
    },
};

use super::window::{WindowImpl, WindowRenderContext};

pub struct HelpWindow {
    should_pause_game: bool,
}

/// Creates a help window from the specified help definition.
pub fn new_help_window(help_def: &HelpDef, surface_size: UVec2) -> Window {
    let size = help_def.dimensions.unwrap_or(IVec2::new(380, 180));
    let pos = help_def
        .position
        .unwrap_or(surface_size.as_ivec2() / 2 - size / 2);

    let should_pause_game = !help_def.do_not_pause_game;

    let mut common = WindowCommon::new(Rect::new(pos, size));
    common.is_modal = should_pause_game;
    common.is_always_on_top = true;

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

    common.widgets.add(list_box);

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
        button.custom_color = Some(u32_to_color(0xff19ff19));

        common.widgets.add(button);

        let mut button = Box::new(TextButtonWidget::new(
            Rect::new(
                IVec2::new(button_width * 2 + 48, size.y - 36),
                IVec2::new(button_width, 20),
            ),
            help_def.confirmation_text_2.as_ref().unwrap(),
        ));
        button.font = Font::TwelvePoint;
        button.custom_color = Some(u32_to_color(0xff19ff19));

        common.widgets.add(button);
    }

    Window::new(common, Box::new(HelpWindow { should_pause_game }))
}

impl WindowImpl for HelpWindow {
    fn render(
        &mut self,
        common: &mut WindowCommon,
        context: &mut WindowRenderContext<'_>,
        render_items: &mut WindowRenderItems,
    ) {
        // TODO: The original help window render has first-frame special
        // handling. It does not draw the translucent fills until
        // `m_render_frame_count != 0`, and it increments that counter at the
        // end of each frame.
        //
        // TODO: The original also pauses the game from this render path when
        // `m_pause_game` is true. Mirror that once game pause state is wired
        // into the immediate-mode UI path.
        if self.should_pause_game {
            // Render modal background.
            // Render_Solid_Rect(0,0,g_renderer->m_screen_width,g_renderer->m_screen_height,0x50000000);
            render_items.render_solid_rect(
                Rect::from_size(context.window_renderer.ui_size().as_ivec2()),
                u32_to_color(0x50000000),
            );

            // Render an inner dark rect.
            // Render_Solid_Rect(left + 16,top + 16,width + -32,height + -68,0x80000000);
            render_items.render_solid_rect(
                common
                    .rect
                    .offset(IVec2::splat(16))
                    .grow(-IVec2::new(32, 68)),
                u32_to_color(0x80000000),
            );
        }

        // Render a border around the contents.
        // Render_Single_Pixel_Border(left + -1,top + -1,width + 1 + left,top + 1 + height,0xff19ff19);
        render_items.render_border(
            common.rect.offset(-IVec2::ONE).grow(IVec2::splat(2)),
            1,
            u32_to_color(0xff19ff19),
        );

        // Render the background for the window.
        // Render_Solid_Rect(left,top,width,height,0x50000000);
        render_items.render_solid_rect(common.rect, u32_to_color(0x50000000));

        // TODO: Render the help pointer only for defs that specify one. The
        // quit-confirmation dialog likely does not need this, but other help
        // windows do.

        const DELTA_TIME_MS: i32 = 10;

        common
            .widgets
            .render(common.rect.position, DELTA_TIME_MS, context, render_items);
    }
}
