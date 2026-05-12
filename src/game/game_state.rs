use std::path::PathBuf;

use glam::UVec2;

use crate::{
    engine::{
        assets::AssetError,
        input::InputEvent,
        renderer::{RenderContext, RenderTarget, SurfaceDesc},
    },
    game::{
        assets::config::campaign_def::CampaignDefs,
        config::load_config,
        sim::SimWorld,
        ui::windows::{
            bottombar::BottomBarWindow, main_menu::MainMenuWindow,
            window_manager::WindowManager,
        },
        world_layer::WorldLayer,
    },
};

use super::ui::windows::actions::WindowManagerAction;

/// The main state of the game.
pub struct GameState {
    campaign_defs: CampaignDefs,

    surface_size: UVec2,
    surface_format: wgpu::TextureFormat,
    world_layer: Option<WorldLayer>,
    window_manager: WindowManager,
}

impl GameState {
    pub fn new(surface_desc: &SurfaceDesc) -> Result<Self, AssetError> {
        let campaign_defs = load_config(PathBuf::from("config").join("campaign_defs.txt"))?;

        let mut window_manager = WindowManager::new(surface_desc)?;

        let main_menu_window = Box::new(MainMenuWindow::new(&window_manager)?);
        window_manager.push(main_menu_window);

        Ok(Self {
            campaign_defs,
            surface_size: surface_desc.size,
            surface_format: surface_desc.format,
            world_layer: None,
            window_manager,
        })
    }

    pub fn resize(&mut self, size: UVec2) {
        self.surface_size = size;
        if let Some(world_layer) = &mut self.world_layer {
            world_layer.resize(size);
        }
        self.window_manager.resize(size);
    }

    pub fn input(&mut self, event: &InputEvent) {
        let ui_consumed = self.window_manager.input(event);

        // TODO: This shouldn't reach into window manager's internals.
        let mut actions = std::mem::take(&mut self.window_manager.window_manager_context.actions);
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
        self.window_manager.update(delta_time);
    }

    pub fn render(&mut self, render_context: &mut RenderContext, render_target: &RenderTarget) {
        if let Some(world_layer) = &mut self.world_layer {
            world_layer.render(render_context, render_target);
        } else {
            clear_render_target(render_context, render_target);
        }

        self.window_manager.render(render_context, render_target);
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

        self.window_manager.clear();
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
        let bottom_bar_window = Box::new(BottomBarWindow::new(&self.window_manager)?);
        self.window_manager.push(bottom_bar_window);
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
