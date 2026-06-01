use std::{path::PathBuf, sync::Arc};

use glam::IVec2;

use crate::{
    engine::assets::AssetError,
    game::{
        globals,
        ui::{
            Rect,
            widgets::{main_menu_button::create_main_menu_button, text_button::TextButtonWidget},
            windows::{
                actions::WindowManagerAction,
                window::{Window, WindowImpl},
                window_manager::WindowLayoutContext,
            },
        },
    },
};

pub struct MainMenuWindow;

pub fn new_main_menu_window(context: &WindowLayoutContext) -> Result<Window, AssetError> {
    let window_base = globals::window_manager().get_window_base("main_menu")?;

    let rect = window_base.resolve_layout_rect(context);

    let mut window = Window::from_window_base(
        Arc::clone(&window_base),
        context,
        rect,
        Box::new(MainMenuWindow),
    )?;

    window.common.widgets.add(Box::new(
        TextButtonWidget::new(
            Rect::new(IVec2::new(10, 10), IVec2::new(100, 30)),
            "Training",
        )
        .with_action(WindowManagerAction::StartCampaign(String::from("training"))),
    ));

    let button_offset = IVec2::new(
        window_base
            .ivars
            .get("button_offset_x")
            .and_then(|ivar| window_base.resolve_ivar(ivar, context))
            .unwrap_or(0),
        window_base
            .ivars
            .get("button_offset_y")
            .and_then(|ivar| window_base.resolve_ivar(ivar, context))
            .unwrap_or(0),
    );

    let shadow_offset = IVec2::new(
        window_base
            .ivars
            .get("shadow_offset_x")
            .and_then(|ivar| window_base.resolve_ivar(ivar, context))
            .unwrap_or(0),
        window_base
            .ivars
            .get("shadow_offset_y")
            .and_then(|ivar| window_base.resolve_ivar(ivar, context))
            .unwrap_or(0),
    );

    const BUTTONS: &[(&str, &str, usize, &str, usize)] = &[
        (
            "b_new_game",
            "interface_elements_14",
            0,
            "interface_elements_14",
            1,
        ),
        (
            "b_training",
            "interface_elements_17",
            0,
            "interface_elements_17",
            1,
        ),
        (
            "b_options",
            "interface_elements_15",
            0,
            "interface_elements_15",
            1,
        ),
        (
            "b_exit",
            "interface_elements_15",
            3,
            "interface_elements_15",
            4,
        ),
        (
            "b_load_game",
            "interface_elements_13",
            0,
            "interface_elements_13",
            1,
        ),
        (
            "b_multiplayer",
            "interface_elements_14",
            3,
            "interface_elements_14",
            4,
        ),
        (
            "b_intro",
            "interface_elements_13",
            3,
            "interface_elements_13",
            4,
        ),
    ];

    let Some(bullet_sprite) = globals::sprites().get_handle_by_name("interface_elements_16") else {
        return Err(AssetError::custom(PathBuf::new(), "bullet_sprite"));
    };
    let bullet_frame = 3;

    for (button, text_sprite, text_frame, shadow_sprite, shadow_frame) in BUTTONS.iter().cloned() {
        let Some(button_advice) = window_base.button_advices.get(button) else {
            continue;
        };

        let pos = IVec2::new(
            window_base
                .resolve(&button_advice.x, context)
                .unwrap_or_default(),
            window_base
                .resolve(&button_advice.y, context)
                .unwrap_or_default(),
        );

        let Some(text_sprite) = globals::sprites().get_handle_by_name(text_sprite) else {
            continue;
        };
        let Some(shadow_sprite) = globals::sprites().get_handle_by_name(shadow_sprite) else {
            continue;
        };

        window.common.widgets.add(create_main_menu_button(
            pos,
            bullet_sprite,
            bullet_frame,
            text_sprite,
            text_frame,
            shadow_sprite,
            shadow_frame,
            button_offset,
            shadow_offset,
        ));
    }

    Ok(window)
}

impl WindowImpl for MainMenuWindow {}
