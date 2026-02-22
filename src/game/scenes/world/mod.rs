use bevy_ecs::prelude::*;
use glam::UVec2;
use winit::keyboard::KeyCode;

use crate::{
    engine::{
        assets::AssetError,
        input::InputState,
        renderer::{Frame, Renderer},
        scene::Scene,
        shader_cache::ShaderCache,
    },
    game::{
        AssetLoader, AssetReader,
        config::CampaignDef,
        scenes::world::{
            extract::RenderSnapshot,
            game_mode::GameMode,
            render::{RenderBindings, RenderLayouts, RenderTargets},
            sim_world::{Camera, ecs::Viewport, init_sim_world},
            systems::{SimulationControl, Time},
        },
    },
};

pub mod actions;
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
    sim_world: World,
    render_targets: RenderTargets,
    // shader_cache: ShaderCache,
    bindings: RenderBindings,

    // Systems
    systems: systems::Systems,

    /// A cache of the last surface size, set during resize and used during render.
    surface_size: UVec2,

    game_mode: GameMode,

    fps_history: Vec<FrameTime>,
    fps_history_cursor: usize,

    /// When true, simulation systems stop updating unless a step is requested.
    simulation_paused: bool,
    /// Fixed delta-time used by the single-step debug control.
    simulation_step_delta_time: f32,
    /// Number of pending fixed-step simulation ticks to execute.
    pending_simulation_steps: u32,

    sequence_to_play: String,
}

impl WorldScene {
    pub fn new(
        renderer: &Renderer,
        surface_size: UVec2,
        surface_format: wgpu::TextureFormat,
        campaign_def: CampaignDef,
    ) -> Result<Self, AssetError> {
        tracing::info!("Loading campaign \"{}\"...", campaign_def.title);

        let fps_history = vec![FrameTime::default(); 100];
        let fps_history_cursor = 0;

        let assets = AssetLoader::new()?;

        let mut sim_world = World::default();

        init_sim_world(&mut sim_world, assets, &campaign_def)?;

        let render_targets = RenderTargets::new(renderer, surface_size, surface_format);

        let mut layouts = RenderLayouts::new();

        let mut shader_cache = ShaderCache::default();

        let bindings = RenderBindings::new(renderer, &mut layouts);

        let systems = systems::Systems::new(
            renderer,
            &render_targets,
            &mut layouts,
            &mut shader_cache,
            &mut sim_world,
        );

        Ok(Self {
            sim_world,
            render_targets,
            bindings,

            systems,

            surface_size: UVec2::ZERO,

            game_mode: GameMode::Editor,

            fps_history,
            fps_history_cursor,

            simulation_paused: false,
            simulation_step_delta_time: 1.0 / 30.0,
            pending_simulation_steps: 0,

            sequence_to_play: String::from("MSEQ_WALK"),
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
        let run_simulation = if self.simulation_paused {
            if self.pending_simulation_steps > 0 {
                self.pending_simulation_steps -= 1;
                true
            } else {
                false
            }
        } else {
            true
        };
        let simulation_delta_time = if self.simulation_paused && run_simulation {
            self.simulation_step_delta_time
        } else {
            delta_time
        };

        // Run systems
        {
            // Update Time.
            {
                let mut time = self.sim_world.resource_mut::<Time>();
                if run_simulation {
                    time.next_frame(simulation_delta_time);
                } else {
                    // Keep camera/input responsiveness while pausing simulation.
                    time.delta_time = delta_time;
                }
            }

            {
                let mut control = self.sim_world.resource_mut::<SimulationControl>();
                control.run_update = run_simulation;
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
                let assets = self.sim_world.resource::<AssetReader>();
                self.systems
                    .prepare(assets, &mut self.bindings, renderer, render_snapshot);
                frame_time.prepare = (std::time::Instant::now() - start).as_secs_f64();
            }

            // Queue
            {
                let start = std::time::Instant::now();
                let render_snapshot = self.sim_world.resource::<RenderSnapshot>();
                self.systems
                    .queue(&self.render_targets, &self.bindings, render_snapshot, frame);
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

                    ui.heading("Simulation");
                    ui.horizontal(|ui| {
                        let toggle_label = if self.simulation_paused {
                            "Play"
                        } else {
                            "Pause"
                        };
                        if ui.button(toggle_label).clicked() {
                            self.simulation_paused = !self.simulation_paused;
                        }
                        if ui.button("Step").clicked() {
                            self.pending_simulation_steps =
                                self.pending_simulation_steps.saturating_add(1);
                        }
                        ui.label("Step dt");
                        ui.add(
                            egui::DragValue::new(&mut self.simulation_step_delta_time)
                                .range(1.0 / 240.0..=0.25)
                                .speed(0.0005)
                                .fixed_decimals(4),
                        );
                        ui.label(if self.simulation_paused {
                            "Paused"
                        } else {
                            "Running"
                        });
                    });

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
                    let mut render_snapshot = self.sim_world.resource_mut::<RenderSnapshot>();
                    ui.heading("Terrain");
                    ui.checkbox(
                        &mut render_snapshot.terrain.render_wireframe,
                        "Render terrain wireframe",
                    );
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

        {
            let world_interaction = self
                .sim_world
                .resource::<systems::world_interaction::WorldInteraction>();

            if let Some(selected_entity) = world_interaction.selected_entity {
                egui::Window::new("Selected")
                    .resizable(false)
                    .show(ctx, |ui| {
                        use crate::game::hash;

                        if let Some(spawn_info) =
                            self.sim_world.get::<sim_world::SpawnInfo>(selected_entity)
                        {
                            ui.label(format!("{} ({})", spawn_info._title, spawn_info._name));
                        }

                        {
                            use crate::game::scenes::world::sim_world::sequences::MotionController;

                            if let Some(mc) =
                                self.sim_world.get::<MotionController>(selected_entity)
                            {
                                ui.label(format!(
                                    "Transition check state: {:?}",
                                    mc.transition_check_state()
                                ));
                            }
                        }

                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut self.sequence_to_play);
                            if ui.button("Request").clicked() {
                                let request = sim_world::sequences::MotionSequenceRequest {
                                    entity: selected_entity,
                                    sequence_hash: hash(self.sequence_to_play.as_str()),
                                    playback_speed: 1.0,
                                    ..Default::default()
                                };
                                self.sim_world.trigger(request);
                            }
                        });

                        ui.horizontal_wrapped(|ui| {
                            ui.label("States:");
                            for (label, sequence_name) in [
                                ("Stand", "MSEQ_STAND"),
                                ("Crouch", "MSEQ_CROUCH"),
                                ("Prone", "MSEQ_PRONE"),
                                ("OnBack", "MSEQ_ON_BACK"),
                                ("Sit", "MSEQ_SIT"),
                                ("Scuba", "MSEQ_SCUBA"),
                            ] {
                                if ui.button(label).clicked() {
                                    self.sequence_to_play = sequence_name.to_string();
                                    let request = sim_world::sequences::MotionSequenceRequest {
                                        entity: selected_entity,
                                        sequence_hash: hash(sequence_name),
                                        playback_speed: 1.0,
                                        ..Default::default()
                                    };
                                    self.sim_world.trigger(request);
                                }
                            }
                        });

                        if let Some(mc) = self
                            .sim_world
                            .get_mut::<sim_world::sequences::MotionController>(selected_entity)
                        {
                            mc.pending.iter().for_each(|context| {
                                ui.label(format!(
                                    "{} ({:.02})",
                                    &context.motion_info.motion.name, context.playback_speed
                                ));
                            });
                        }
                    });
            }
        }

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
