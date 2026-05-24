use std::path::PathBuf;

use glam::UVec2;

use super::ui::windows::actions::WindowManagerAction;
use crate::{
    engine::{
        assets::AssetError,
        input::InputEvent,
        renderer::{RenderContext, RenderTarget, SurfaceDesc},
    },
    game::{
        assets::config::campaign_def::CampaignDefs,
        config::load_config,
        globals,
        sim::SimWorld,
        ui::{
            render::window_renderer::{UiMode, WindowRenderer},
            windows::{
                bottombar::new_bottombar_window, main_menu::new_main_menu_window,
                window_manager::WindowLayoutContext,
            },
        },
        world_layer::WorldLayer,
    },
};

/// The main state of the game.
pub struct GameState {
    campaign_defs: CampaignDefs,

    surface_size: UVec2,
    surface_format: wgpu::TextureFormat,
    world_layer: Option<WorldLayer>,
    window_renderer: WindowRenderer,
}

impl GameState {
    pub fn new(surface_desc: &SurfaceDesc) -> Result<Self, AssetError> {
        let campaign_defs = load_config(PathBuf::from("config").join("campaign_defs.txt"))?;

        let window_renderer = WindowRenderer::new(surface_desc);

        let context = WindowLayoutContext {
            screen_dx: surface_desc.size.x as i32,
            screen_dy: surface_desc.size.y as i32,
        };

        let main_menu_window = new_main_menu_window(&context)?;
        globals::window_manager().push(main_menu_window);

        Ok(Self {
            campaign_defs,
            surface_size: surface_desc.size,
            surface_format: surface_desc.format,
            world_layer: None,
            window_renderer,
        })
    }

    pub fn resize(&mut self, size: UVec2, scale_factor: f32) {
        self.surface_size = size;
        if let Some(world_layer) = &mut self.world_layer {
            world_layer.resize(size);
        }
        globals::window_manager().resize(size, scale_factor, &mut self.window_renderer);
    }

    pub fn input(&mut self, event: &InputEvent) {
        let ui_consumed = globals::window_manager().input(event, &self.window_renderer);

        // TODO: This shouldn't reach into window manager's internals.
        let mut actions =
            std::mem::take(&mut globals::window_manager().window_manager_context.actions);
        for action in actions.drain(..) {
            match action {
                WindowManagerAction::Quit => tracing::info!("Quit game!"),
                WindowManagerAction::StartCampaign(name) => match self.start_campaign(&name) {
                    Ok(_) => {}
                    Err(err) => {
                        tracing::error!("Could not start campaign {name} - {err}");
                    }
                },
            }
        }

        if !ui_consumed && let Some(world_layer) = &mut self.world_layer {
            world_layer.input(event);
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        if let Some(world_layer) = &mut self.world_layer {
            world_layer.update(delta_time);
        }
        globals::window_manager().update(delta_time);
    }

    pub fn render(&mut self, render_context: &mut RenderContext, render_target: &RenderTarget) {
        if let Some(world_layer) = &mut self.world_layer {
            world_layer.render(render_context, render_target);
        } else {
            clear_render_target(render_context, render_target);
        }

        globals::window_manager().render(render_context, render_target, &mut self.window_renderer);
    }

    #[cfg(feature = "egui")]
    pub fn debug_panel(&mut self, egui: &egui::Context, frame_index: u64) {
        let _ = (egui, frame_index);
    }

    fn start_campaign(&mut self, name: &str) -> Result<(), AssetError> {
        tracing::info!("Starting campaign: {name}");

        let Some(campaign_def) = self
            .campaign_defs
            .campaign_defs
            .iter()
            .find(|c| c.base_name.eq_ignore_ascii_case(name))
        else {
            return Err(AssetError::Custom(
                PathBuf::new(),
                String::from("Campaign not found!"),
            ));
        };

        let sim = SimWorld::new(campaign_def)?;

        globals::window_manager().clear();
        globals::window_manager().set_ui_mode(UiMode::Native, &mut self.window_renderer);
        self.world_layer = Some(WorldLayer::new(self.surface_size, self.surface_format, sim));
        self.spawn_game_ui()?;

        Ok(())
    }

    /// Mirrors the original engine's `Spawn_Game_UI` (`0x004dde00`), which the
    /// `Big_Switch` campaign-start events call right after constructing the
    /// terrain window. The original creates Window_Bottom_Bar,
    /// Window_Inventory_Bar, and Window_Command_Pad — only the first is wired
    /// up here so far.
    fn spawn_game_ui(&mut self) -> Result<(), AssetError> {
        let bottom_bar_window = new_bottombar_window(self.window_renderer.ui_size().as_ivec2())?;
        globals::window_manager().push(bottom_bar_window);
        Ok(())
    }
}

/// Clears the swapchain target when no native world layer is present.
fn clear_render_target(render_context: &mut RenderContext, render_target: &RenderTarget) {
    render_context
        .encoder
        .begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("game_state_surface_clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &render_target.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
}
