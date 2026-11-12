use std::{path::PathBuf, sync::Arc};

use glam::{IVec2, Vec2, Vec4};

use crate::{
    engine::assets::AssetError,
    game::{
        config::window_base::WindowBase,
        globals,
        ui::{
            Rect,
            render::window_renderer::WindowRenderItems,
            widgets::main_menu_button::{ButtonLayer, create_main_menu_button},
            windows::{
                actions::WindowManagerAction,
                window::{Window, WindowBehavior, WindowBuilder},
                window_manager::WindowLayoutContext,
            },
        },
    },
};

use super::window::{WindowCommon, WindowRenderContext};

impl WindowBase {
    fn get_button_offset(&self, context: &WindowLayoutContext) -> IVec2 {
        IVec2::new(
            self.ivars
                .get("button_offset_x")
                .and_then(|ivar| self.resolve_ivar(ivar, context))
                .unwrap_or(0),
            self.ivars
                .get("button_offset_y")
                .and_then(|ivar| self.resolve_ivar(ivar, context))
                .unwrap_or(0),
        )
    }

    fn get_shadow_offset(&self, context: &WindowLayoutContext) -> IVec2 {
        IVec2::new(
            self.ivars
                .get("shadow_offset_x")
                .and_then(|ivar| self.resolve_ivar(ivar, context))
                .unwrap_or(0),
            self.ivars
                .get("shadow_offset_y")
                .and_then(|ivar| self.resolve_ivar(ivar, context))
                .unwrap_or(0),
        )
    }
}

#[derive(Debug)]
struct Frame {
    index: usize,
    alpha: f32,
}

pub struct MainMenuBehavior {
    frames: Vec<usize>,
    phase: f64,
}

impl MainMenuBehavior {
    /// Draws the two neighboring frames at the current animation position.
    fn render_frames(&self, window: &WindowCommon, render_items: &mut WindowRenderItems) {
        let count = self.frames.len();
        if count == 0 {
            return;
        }

        let last = (count - 1) as f64;

        // Reflect the phase to travel 0 → last → 0.
        let position = if self.phase <= last {
            self.phase
        } else {
            2.0 * last - self.phase
        };

        let lower = position.floor() as usize;
        let upper = (lower + 1).min(count - 1);
        let blend = (position - lower as f64) as f32;

        // An opaque base plus a translucent overlay produces a crossfade.
        for (frame, alpha) in [(lower, 1.0), (upper, blend)] {
            if alpha <= 0.0 {
                continue;
            }

            let geometry = &window.geometries.tiled[self.frames[frame]];
            render_items.render_textured_rect(
                Rect::new(IVec2::ZERO, geometry.size),
                geometry.texture,
                Vec2::ZERO,
                Vec2::ONE,
                Vec4::new(1.0, 1.0, 1.0, alpha),
            );
        }
    }
}

impl WindowBehavior for MainMenuBehavior {
    fn update(&mut self, window: &mut WindowCommon, delta: f32) {
        window.update_widgets(delta);
        if self.frames.len() < 2 {
            return;
        }

        let cycle = 2.0 * (self.frames.len() - 1) as f64;
        self.phase = (self.phase + f64::from(delta) * 0.4).rem_euclid(cycle);
    }

    fn render(
        &mut self,
        window: &mut WindowCommon,
        context: &mut WindowRenderContext<'_>,
        render_items: &mut WindowRenderItems,
    ) {
        self.render_frames(window, render_items);
        window.render_widgets(context, render_items);
    }
}

pub fn new_main_menu_window(context: &WindowLayoutContext) -> Result<Window, AssetError> {
    let window_base = globals::window_manager().get_window_base("main_menu")?;

    let rect = window_base.resolve_layout_rect(context);

    let mut window = WindowBuilder::new(rect)
        .with_window_base(Arc::clone(&window_base), context)
        .build(|common| {
            Ok(MainMenuBehavior {
                frames: (0..common.geometries.tiled.len()).rev().collect(),
                phase: 0.0,
            })
        })?;

    // let button = Box::new(TextButtonWidget {
    //     widget: Widget {
    //         rect: Rect::new(IVec2::new(10, 10), IVec2::new(100, 30)),
    //         ..Default::default()
    //     },
    //     // action: Some(WindowManagerAction::StartCampaign(String::from("training"))),
    //     text: String::from("Training"),
    //     ..Default::default()
    // });
    // window.widgets.add(button);

    let button_offset = window_base.get_button_offset(context);
    let shadow_offset = window_base.get_shadow_offset(context);

    let buttons = &[
        (
            "b_new_game",
            "interface_elements_14",
            0,
            "interface_elements_14",
            1,
            Some(WindowManagerAction::ShowDifficultySelectionWindow),
        ),
        (
            "b_training",
            "interface_elements_17",
            0,
            "interface_elements_17",
            1,
            Some(WindowManagerAction::StartCampaign(String::from("training"))),
        ),
        (
            "b_options",
            "interface_elements_15",
            0,
            "interface_elements_15",
            1,
            None,
        ),
        (
            "b_exit",
            "interface_elements_15",
            3,
            "interface_elements_15",
            4,
            None,
        ),
        (
            "b_load_game",
            "interface_elements_13",
            0,
            "interface_elements_13",
            1,
            None,
        ),
        (
            "b_multiplayer",
            "interface_elements_14",
            3,
            "interface_elements_14",
            4,
            None,
        ),
        (
            "b_intro",
            "interface_elements_13",
            3,
            "interface_elements_13",
            4,
            None,
        ),
    ];

    let Some(bullet_sprite) = globals::sprites().get_handle_by_name("interface_elements_16") else {
        return Err(AssetError::custom(PathBuf::new(), "bullet_sprite"));
    };
    let bullet_frame = 3;

    for (button, text_sprite, text_frame, shadow_sprite, shadow_frame, action) in
        buttons.iter().cloned()
    {
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

        let bullet = ButtonLayer::new(bullet_sprite, bullet_frame, pos);
        let text = ButtonLayer::new(text_sprite, text_frame, pos + button_offset);
        let shadow = ButtonLayer::new(shadow_sprite, shadow_frame, pos + shadow_offset);

        window
            .common
            .widgets
            .push(create_main_menu_button(pos, bullet, text, shadow, action));
    }

    Ok(window)
}
