use glam::{IVec2, UVec2};

use crate::game::{
    config::help_window_defs::HelpDef,
    ui::{
        EventResult, Rect,
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
    // TODO: Keep the original help window state here once we match it more
    // closely. The Ghidra render path uses fields like the source help def and
    // a per-window render frame count for first-frame special-casing.
    should_pause_game: bool,
}

impl HelpWindow {
    /// Creates a help window from the specified help definition.
    pub fn new(help_def: &HelpDef, surface_size: UVec2) -> Self {
        let size = help_def.dimensions.unwrap_or(IVec2::new(380, 180));
        let pos = help_def
            .position
            .unwrap_or(surface_size.as_ivec2() / 2 - size / 2);

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
            button.custom_color = Some(u32_to_color(0xff19ff19));

            widgets.add(button);

            let mut button = Box::new(TextButtonWidget::new(
                Rect::new(
                    IVec2::new(button_width * 2 + 48, size.y - 36),
                    IVec2::new(button_width, 20),
                ),
                help_def.confirmation_text_2.as_ref().unwrap(),
            ));
            button.font = Font::TwelvePoint;
            button.custom_color = Some(u32_to_color(0xff19ff19));

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
    fn is_modal(&self) -> bool {
        self.should_pause_game
    }

    fn is_visible(&self) -> bool {
        true
    }

    fn is_always_on_top(&self) -> bool {
        true
    }

    fn wants_input(&self) -> bool {
        true
    }

    fn hit_test(&self, position: IVec2) -> bool {
        let bottom_right = self.rect.bottom_right();
        position.x >= self.rect.position.x
            && position.y >= self.rect.position.y
            && position.x < bottom_right.x
            && position.y < bottom_right.y
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn on_primary_mouse_down(&mut self, mouse: IVec2) -> EventResult {
        self.widgets.on_primary_mouse_down(mouse)
    }

    fn on_secondary_mouse_down(&mut self, _mouse: IVec2) -> EventResult {
        // TODO: Confirm whether right-click should be ignored or routed to the
        // child widgets for help-window button handling.
        EventResult::Ignore
    }

    fn on_primary_mouse_up(&mut self, mouse: IVec2) -> EventResult {
        self.widgets.on_primary_mouse_up(mouse)
    }

    fn on_mouse_wheel(&mut self, mouse: IVec2, wheel_steps: i32) -> EventResult {
        self.widgets.on_mouse_wheel(mouse, wheel_steps)
    }

    fn render(&mut self, window_renderer: &WindowRenderer, render_items: &mut WindowRenderItems) {
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
                Rect::from_size(window_renderer.surface_size().as_ivec2()),
                u32_to_color(0x50000000),
            );

            // Render an inner dark rect.
            // Render_Solid_Rect(left + 16,top + 16,width + -32,height + -68,0x80000000);
            render_items.render_solid_rect(
                self.rect.offset(IVec2::splat(16)).grow(-IVec2::new(32, 68)),
                u32_to_color(0x80000000),
            );
        }

        // Render a border around the contents.
        // Render_Single_Pixel_Border(left + -1,top + -1,width + 1 + left,top + 1 + height,0xff19ff19);
        render_items.render_border(
            self.rect.offset(-IVec2::ONE).grow(IVec2::splat(2)),
            1,
            u32_to_color(0xff19ff19),
        );

        // Render the background for the window.
        // Render_Solid_Rect(left,top,width,height,0x50000000);
        render_items.render_solid_rect(self.rect, u32_to_color(0x50000000));

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
