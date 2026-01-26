use bevy_ecs::prelude::*;
use glam::UVec2;
use winit::keyboard::KeyCode;

use crate::{
    engine::{
        assets::AssetError,
        input::InputState,
        renderer::{Frame, Renderer},
        scene::Scene,
    },
    game::{
        AssetLoader, AssetReader,
        config::CampaignDef,
        scenes::world::{
            extract::RenderSnapshot,
            game_mode::GameMode,
            render::{RenderStore, RenderTargets, RenderWorld},
            sim_world::{Camera, ecs::Viewport, init_sim_world},
            systems::Time,
        },
    },
};

pub mod actions;
pub mod animation;
mod extract;
mod game_mode;
mod render;
pub mod sim_world;
mod systems;

#[derive(Clone, Copy, Debug, Default)]
struct FrameTime {
    update: f64,
    extract: f64,
    prepare: f64,
    queue: f64,
}

/// The [Scene] that renders the ingame world view.
pub struct WorldScene {
    assets: AssetReader,

    sim_world: World,
    render_targets: RenderTargets,
    render_worlds: [RenderWorld; Self::RENDER_FRAME_COUNT],

    // Systems
    systems: systems::Systems,

    /// A cache of the last surface size, set during resize and used during render.
    surface_size: UVec2,

    game_mode: GameMode,

    fps_history: Vec<FrameTime>,
    fps_history_cursor: usize,
}

impl WorldScene {
    const RENDER_FRAME_COUNT: usize = 3;

    pub fn new(
        renderer: &Renderer,
        surface_size: UVec2,
        surface_format: wgpu::TextureFormat,
        campaign_def: CampaignDef,
    ) -> Result<Self, AssetError> {
        tracing::info!("Loading campaign \"{}\"...", campaign_def.title);

        let fps_history = vec![FrameTime::default(); 100];
        let fps_history_cursor = 0;

        let mut assets = AssetLoader::new()?;

        let mut sim_world = World::default();

        init_sim_world(&mut sim_world, &mut assets, &campaign_def)?;

        let render_targets = RenderTargets::new(renderer, surface_size, surface_format);

        let mut render_store = RenderStore::new(renderer);

        let render_worlds = [
            RenderWorld::new(0, renderer, &render_store),
            RenderWorld::new(1, renderer, &render_store),
            RenderWorld::new(2, renderer, &render_store),
        ];

        // All assets should be loaded now, turn the [AssetLoader] into an [AssetReader].
        let assets = assets.into_reader();

        let systems = systems::Systems::new(
            &assets,
            renderer,
            &render_targets,
            &mut render_store,
            &mut sim_world,
        );

        Ok(Self {
            assets,

            sim_world,
            render_targets,
            render_worlds,

            systems,

            surface_size: UVec2::ZERO,

            game_mode: GameMode::Editor,

            fps_history,
            fps_history_cursor,
        })
    }

    pub fn in_editor(&self) -> bool {
        matches!(self.game_mode, GameMode::Editor)
    }
}

impl Scene for WorldScene {
    fn resize(&mut self, size: UVec2) {
        self.surface_size = size;

        let [width, height] = size.as_vec2().to_array();
        let aspect = width / height.max(1.0);

        self.sim_world.resource_mut::<Viewport>().resize(size);

        for mut camera in self
            .sim_world
            .query::<&mut Camera>()
            .query_mut(&mut self.sim_world)
        {
            camera.aspect_ratio = aspect;
        }
    }

    fn update(&mut self, delta_time: f32, input: &InputState) {
        let frame_time = &mut self.fps_history[self.fps_history_cursor];

        // Run systems
        {
            // Update Time.
            {
                let mut time = self.sim_world.resource_mut::<Time>();
                time.next_frame(delta_time);
            }

            {
                let mut res = self.sim_world.resource_mut::<InputState>();
                *res = input.clone();
            }

            let start = std::time::Instant::now();
            self.systems.update(&mut self.sim_world);
            frame_time.update = (std::time::Instant::now() - start).as_secs_f64();
        }

        if input.key_just_pressed(KeyCode::Backquote) {
            self.game_mode = if self.in_editor() {
                GameMode::Game
            } else {
                GameMode::Editor
            }
        }
    }

    fn render(&mut self, renderer: &Renderer, frame: &mut Frame) {
        if self.render_targets.surface_size != self.surface_size {
            self.render_targets.resize(renderer, self.surface_size);
        }

        let render_world =
            &mut self.render_worlds[frame.frame_index as usize % Self::RENDER_FRAME_COUNT];

        let frame_time = &mut self.fps_history[self.fps_history_cursor];

        // Systems
        {
            // Extract
            {
                let start = std::time::Instant::now();
                self.systems.extract(&mut self.sim_world);
                frame_time.extract = (std::time::Instant::now() - start).as_secs_f64();
            }

            // Prepare
            {
                // Make sure the geometry buffer is the correct size.
                if frame.size != self.render_targets.geometry_buffer.size {
                    self.render_targets
                        .geometry_buffer
                        .resize(&renderer.device, frame.size);
                }

                let start = std::time::Instant::now();
                let render_snapshot = self.sim_world.resource::<RenderSnapshot>();
                self.systems
                    .prepare(&self.assets, render_world, renderer, render_snapshot);
                frame_time.prepare = (std::time::Instant::now() - start).as_secs_f64();
            }

            // Queue
            {
                let start = std::time::Instant::now();
                let render_snapshot = self.sim_world.resource::<RenderSnapshot>();
                self.systems
                    .queue(&self.render_targets, render_world, render_snapshot, frame);
                frame_time.queue = (std::time::Instant::now() - start).as_secs_f64();
            }

            self.sim_world.clear_trackers();
            self.sim_world.increment_change_tick();
        }

        self.fps_history_cursor = (self.fps_history_cursor + 1) % self.fps_history.len();
    }

    #[cfg(feature = "egui")]
    fn debug_panel(&mut self, ctx: &egui::Context, frame_index: u64) {
        use egui::widgets::Slider;

        if !self.in_editor() {
            return;
        }

        egui::Window::new("World")
            .default_open(true)
            .show(ctx, |ui| {
                /*
                ui.heading("Camera");
                ui.vertical(|ui| {
                    use crate::game::scenes::world::sim_world::ActiveCamera;

                    let mut state = self.sim_world.state_mut();

                    ui.horizontal(|ui| {
                        use crate::game::scenes::world::sim_world::ActiveCamera;

                        ui.selectable_value(&mut state.active_camera, ActiveCamera::Game, "Game");
                        ui.selectable_value(&mut state.active_camera, ActiveCamera::Debug, "Debug");
                    });

                    match state.active_camera {
                        ActiveCamera::Game => {}
                        ActiveCamera::Debug => {
                            egui::Grid::new("debug_camera")
                                .num_columns(2)
                                .show(ui, |ui| {
                                    ui.label("Speed");
                                    let value = &mut self
                                        .systems
                                        .camera_system
                                        .debug_controller
                                        .movement_speed;
                                    ui.add(
                                        egui::widgets::Slider::new(value, 500.0..=10_000.0)
                                            .fixed_decimals(0),
                                    );
                                    ui.end_row();

                                    ui.label("Sensitivity");
                                    let value = &mut self
                                        .systems
                                        .camera_system
                                        .debug_controller
                                        .mouse_sensitivity;
                                    ui.add(
                                        egui::widgets::Slider::new(value, 0.1..=1.0).step_by(0.01),
                                    );
                                    ui.end_row();
                                });
                        }
                    }
                });
                */

                {
                    use crate::game::scenes::world::sim_world::SimWorldState;

                    let mut state = self.sim_world.resource_mut::<SimWorldState>();

                    ui.heading("Environment");
                    ui.horizontal(|ui| {
                        ui.label("Time of day");
                        ui.add(
                            Slider::new(&mut state.time_of_day, 0.0..=24.0)
                                .drag_value_speed(0.01)
                                .fixed_decimals(2),
                        );
                    });
                }

                // Terrain
                {
                    ui.heading("Terrain");
                    ui.checkbox(
                        &mut self
                            .systems
                            .world_renderer
                            .terrain_pipeline
                            .debug_render_terrain_wireframe,
                        "Render terrain wireframe",
                    );
                }

                // Objects
                {
                    ui.heading("Objects");
                }
            });

        egui::Window::new("Timings")
            .resizable(false)
            .default_open(false)
            .show(ctx, |ui| {
                ui.set_min_size(egui::Vec2::new(400.0, 300.0));
                let (rect, _resp) =
                    ui.allocate_exact_size(egui::vec2(400.0, 300.0), egui::Sense::hover());

                let painter = ui.painter_at(rect);

                let bar_width = rect.width() / self.fps_history.len() as f32;

                let scale = rect.height() as f64 / (1.0 / 288.0);

                const COLORS: &[egui::Color32] = &[
                    egui::Color32::from_rgb(255, 0, 0),   // input
                    egui::Color32::from_rgb(0, 255, 0),   // update
                    egui::Color32::from_rgb(0, 0, 255),   // extract
                    egui::Color32::from_rgb(255, 255, 0), // prepare
                    egui::Color32::from_rgb(255, 0, 255), // queue
                ];

                for i in 0..self.fps_history.len() {
                    let frame_time =
                        self.fps_history[(self.fps_history_cursor + i) % self.fps_history.len()];

                    let left = rect.left() + i as f32 * bar_width;
                    let right = rect.left() + (i + 1) as f32 * bar_width;

                    let mut bottom = rect.bottom();

                    for (c, value) in [
                        frame_time.update,
                        frame_time.extract,
                        frame_time.prepare,
                        frame_time.queue,
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let top = bottom - (value * scale) as f32;
                        let bar_rect = egui::Rect::from_min_max(
                            egui::Pos2::new(left, top),
                            egui::Pos2::new(right, bottom),
                        );
                        painter.rect_filled(bar_rect, 0.0, COLORS[c % COLORS.len()]);
                        bottom = top;
                    }
                }

                let line_pos = rect.bottom() - (rect.bottom() - rect.top()) * 0.5;
                painter.line(
                    vec![
                        egui::Pos2::new(rect.left(), line_pos),
                        egui::Pos2::new(rect.right(), line_pos),
                    ],
                    egui::Stroke::new(2.0, egui::Color32::BLUE),
                )
            });

        egui::Window::new("Stats").show(ctx, |ui| {
            let render_snapshot = self.sim_world.resource::<RenderSnapshot>();

            ui.horizontal(|ui| {
                ui.label("Frame index");
                ui.label(format!("{frame_index}"));
            });
            ui.horizontal(|ui| {
                ui.label("Visible chunks");
                ui.label(format!("{}", render_snapshot.terrain.chunks.len()));
            });
            ui.horizontal(|ui| {
                ui.label("Visible strata");
                ui.label(format!("{}", render_snapshot.terrain.strata.len()));
            });
            ui.horizontal(|ui| {
                ui.label("Visible objects");
                ui.label(format!("{}", render_snapshot.models.models.len()));
            });
            ui.horizontal(|ui| {
                ui.label("Gizmo vertices");
                ui.label(format!("{}", render_snapshot.gizmos.vertices.len()));
            });
        });

        // TODO: Should probably move somewhere.
        /*
        if !self.sim_world.state().selected_objects.is_empty() {
            egui::Window::new("Selected")
                .resizable(false)
                .default_width(400.0)
                .show(ctx, |ui| {
                    use crate::{
                        engine::storage::Handle,
                        game::scenes::world::sim_world::{Object, ObjectData},
                    };

                    let mut selected: Vec<Handle<Object>> = self
                        .sim_world
                        .state()
                        .selected_objects
                        .iter()
                        .cloned()
                        .collect();

                    for handle in selected.drain(..) {
                        let mut objects = self.sim_world.resource_mut::<Objects>();

                        if let Some(object) = objects.get_mut(handle) {
                            use crate::engine::egui_integration::UiExt;

                            ui.h1(format!("{} ({})", &object.title, &object.name));

                            match &mut object.data {
                                ObjectData::Scenery { .. } => {}
                                ObjectData::Biped { order_queue, .. } => {
                                    // sequencer.ui(ui, &state.sequences);
                                    order_queue.ui(ui);
                                }
                                ObjectData::SingleModel { .. } => {}
                            }
                        }
                    }
                });
        }
        */
    }
}
