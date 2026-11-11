use glam::IVec2;

use crate::{
    engine::assets::AssetError,
    game::{
        globals,
        ui::{
            Rect,
            render::window_renderer::{Font, WindowRenderItems},
            u32_to_color,
            widgets::{
                text_button::TextButtonBehavior,
                widget::{Widget, WidgetCommon},
            },
            windows::{
                window::{WindowBehavior, WindowBuilder},
                window_manager::WindowManager,
                window_manager_context::WindowManagerContext,
            },
        },
    },
};

use super::window::{WindowCommon, WindowRenderContext};

pub struct DifficultySelectionWindowBehavior;

impl DifficultySelectionWindowBehavior {
    const SIZE: IVec2 = IVec2::new(360, 250);
}

impl WindowBehavior for DifficultySelectionWindowBehavior {
    fn render(
        &mut self,
        window: &mut WindowCommon,
        context: &mut WindowRenderContext<'_>,
        render_items: &mut WindowRenderItems,
    ) {
        let origin = window.rect.position;

        let screen_size = context.window_renderer.ui_size().as_ivec2();

        let fullscreen_rect = Rect::new(IVec2::ZERO, screen_size);
        render_items.render_solid_rect(fullscreen_rect, u32_to_color(0x40000000));

        render_items.render_border(
            Rect::new(origin - IVec2::ONE, window.rect.size),
            1,
            u32_to_color(0xff333333),
        );
        render_items.render_solid_rect(window.rect, u32_to_color(0x80000000));

        render_items.render_text(
            origin + IVec2::new(8, 12),
            b"Select Difficulty Level:",
            Font::FifteenPoint,
            Some(u32_to_color(0xff19cc19)),
        );

        render_items.render_solid_rect(
            Rect::new(
                origin + IVec2::new(4, 30),
                IVec2::new(Self::SIZE.x - 4 * 2, 1),
            ),
            u32_to_color(0xff19cc19),
        );

        window.render_widgets(context, render_items);
    }
}

pub fn show_difficulty_selection_window() -> Result<(), AssetError> {
    let screen_size = WindowManager::UI_SIZE;

    let mut window = WindowBuilder::new(Rect::new(
        (screen_size - DifficultySelectionWindowBehavior::SIZE) / 2,
        DifficultySelectionWindowBehavior::SIZE,
    ))
    .modal(true)
    .build(|_| Ok(DifficultySelectionWindowBehavior))?;

    // TODO: Highlight on hover flag on buttons.

    let mut add_button = |rect: Rect, text: String| {
        let easy_button = Widget {
            common: WidgetCommon {
                rect,
                ..Default::default()
            },
            behavior: Some(Box::new(TextButtonBehavior {
                text,
                font: Font::FifteenPoint,
                custom_color: Some(u32_to_color(0xff19ff19)),
                ..Default::default()
            })),
        };

        window.common.widgets.push(easy_button);
    };

    add_button(
        Rect::new(IVec2::new(15, 60), IVec2::new(330, 32)),
        "Easy".into(),
    );

    add_button(
        Rect::new(IVec2::new(15, 108), IVec2::new(330, 32)),
        "Normal".into(),
    );

    add_button(
        Rect::new(IVec2::new(15, 156), IVec2::new(330, 32)),
        "Hard".into(),
    );

    add_button(
        Rect::new(IVec2::new(15, 204), IVec2::new(330, 32)),
        "Sinister".into(),
    );

    globals::window_manager().push(window);

    Ok(())
}

/*
1. Pauses the game.
2. Dims the whole screen with black at approximately 25% opacity (0x40000000).
3. Draws a dark-gray, one-pixel border around the centered 360 × 250 dialog.
4. Fills the dialog with black at approximately 50% opacity (0x80000000).
5. Draws the localized heading (0x20) in green, using FONT_15_PT.
6. Draws a green horizontal separator below the heading.
7. Renders four green difficulty buttons: Easy, Normal, Hard, Sinister, each with hover highlighting.
*/
