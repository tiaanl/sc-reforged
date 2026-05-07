use std::{path::PathBuf, sync::Arc};

use glam::UVec2;

use crate::{
    engine::{
        assets::AssetError,
        input::InputEvent,
        renderer::{Gpu, RenderContext, RenderTarget, SurfaceDesc},
    },
    game::{
        assets::{
            config::campaign_def::CampaignDefs, models::Models, motions::Motions, sprites::Sprites,
        },
        config::load_config,
        render::textures::Textures,
        sim::{GameAssets, SimWorld},
        ui::windows::{
            main_menu::MainMenuWindow, window_manager::WindowManager, world::WorldWindow,
        },
    },
};

use super::ui::windows::actions::WindowManagerAction;

/// The main state of the game.
pub struct GameState {
    gpu: Gpu,

    campaign_defs: CampaignDefs,

    models: Arc<Models>,
    motions: Arc<Motions>,
    textures: Arc<Textures>,

    window_manager: WindowManager,
}

impl GameState {
    pub fn new(gpu: Gpu, surface_desc: &SurfaceDesc) -> Result<Self, AssetError> {
        let campaign_defs = load_config(PathBuf::from("config").join("campaign_defs.txt"))?;

        let models = Arc::new(Models::default());
        let motions = Arc::new(Motions::default());
        let textures = Arc::new(Textures::new(gpu.clone()));
        let sprites = Arc::new(Sprites::new(Arc::clone(&textures))?);

        let mut window_manager =
            WindowManager::new(gpu.clone(), surface_desc, Arc::clone(&textures), sprites)?;

        let main_menu_window = Box::new(MainMenuWindow::new(&window_manager)?);
        window_manager.push(main_menu_window);

        // {
        //     let help_window_defs: HelpWindowDefs = load_config(
        //         &file_system,
        //         PathBuf::from("config").join("help_window_defs.txt"),
        //     )?;

        //     if let Some(help_def) = help_window_defs.get("conf_exit_game") {
        //         window_manager.push(Box::new(HelpWindow::new(help_def, surface_desc.size)));
        //     }
        // }

        Ok(Self {
            gpu,
            campaign_defs,
            models,
            motions,
            textures,
            window_manager,
        })
    }

    pub fn resize(&mut self, size: UVec2) {
        self.window_manager.resize(size);
    }

    pub fn input(&mut self, event: &InputEvent) {
        self.window_manager.input(event);

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
    }

    pub fn update(&mut self, delta_time: f32) {
        self.window_manager.update(delta_time);
    }

    pub fn render(&mut self, render_context: &mut RenderContext, render_target: &RenderTarget) {
        self.window_manager
            .render(&self.gpu, render_context, render_target);
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

        let sim = SimWorld::new(
            GameAssets {
                models: Arc::clone(&self.models),
                motions: Arc::clone(&self.motions),
            },
            campaign_def,
        )?;

        self.window_manager.clear();

        let world_window = Box::new(WorldWindow::new(
            self.gpu.clone(),
            Arc::clone(&self.models),
            Arc::clone(&self.textures),
            self.window_manager.window_renderer(),
            UVec2::new(640, 480),
            sim,
        )?);
        self.window_manager.push(world_window);

        Ok(())
    }
}
