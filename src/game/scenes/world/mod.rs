use glam::UVec2;

use crate::{
    engine::prelude::*,
    game::{
        config::CampaignDef,
        data_dir::data_dir,
        scenes::world::{
            game_mode::GameMode,
            render::{RenderStore, RenderWorld},
            sim_world::SimWorld,
        },
    },
};

pub mod actions;
pub mod animation;
mod game_mode;
mod render;
pub mod sim_world;
mod systems;

#[derive(Clone, Copy, Debug, Default)]
struct FrameTime {
    input: f64,
    update: f64,
    extract: f64,
    prepare: f64,
    queue: f64,
}

/// The [Scene] that renders the ingame world view.
pub struct WorldScene {
    sim_world: SimWorld,
    render_worlds: [RenderWorld; Self::RENDER_FRAME_COUNT],
    render_store: RenderStore,

    // Systems
    systems: systems::Systems,

    game_mode: GameMode,

    fps_history: Vec<FrameTime>,
    fps_history_cursor: usize,
}

impl WorldScene {
    const RENDER_FRAME_COUNT: usize = 3;

    pub fn new(campaign_def: CampaignDef, window_size: UVec2) -> Result<Self, AssetError> {
        tracing::info!("Loading campaign \"{}\"...", campaign_def.title);

        let campaign = data_dir().load_campaign(&campaign_def.base_name)?;

        let fps_history = vec![FrameTime::default(); 100];
        let fps_history_cursor = 0;

        let sim_world = SimWorld::new(&campaign_def)?;

        let render_store = RenderStore::new(renderer(), window_size);

        let render_worlds = [
            RenderWorld::new(0, renderer(), &render_store),
            RenderWorld::new(1, renderer(), &render_store),
            RenderWorld::new(2, renderer(), &render_store),
        ];

        let systems = systems::Systems::new(renderer(), &render_store, &sim_world, &campaign);

        Ok(Self {
            sim_world,
            render_worlds,
            render_store,

            systems,

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
    fn resize(&mut self) {
        let size = renderer().surface.size();

        let [width, height] = size.to_array().map(|f| f as f32);
        let aspect = width / height.max(1.0);

        self.sim_world.camera.aspect_ratio = aspect;
    }

    fn update(&mut self, delta_time: f32, input: &InputState) {
        let frame_time = &mut self.fps_history[self.fps_history_cursor];

        // Run systems
        {
            let time = systems::Time { delta_time };
            let viewport_size = renderer().surface.size();

            let start = std::time::Instant::now();
            self.systems
                .input(&mut self.sim_world, &time, input, viewport_size);
            frame_time.input = (std::time::Instant::now() - start).as_secs_f64();

            let start = std::time::Instant::now();
            self.systems.update(&mut self.sim_world, &time);
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

    fn render(&mut self, frame: &mut Frame) {
        let render_world = &mut self.render_worlds[frame.frame_index % Self::RENDER_FRAME_COUNT];

        let frame_time = &mut self.fps_history[self.fps_history_cursor];

        // Systems
        {
            let start = std::time::Instant::now();
            self.systems
                .extract(&mut self.sim_world, &mut self.render_store, render_world);
            frame_time.extract = (std::time::Instant::now() - start).as_secs_f64();

            let start = std::time::Instant::now();
            self.systems
                .prepare(&mut self.render_store, render_world, renderer());
            frame_time.prepare = (std::time::Instant::now() - start).as_secs_f64();

            let start = std::time::Instant::now();
            self.systems.queue(&self.render_store, render_world, frame);
            frame_time.queue = (std::time::Instant::now() - start).as_secs_f64();
        }

        self.fps_history_cursor = (self.fps_history_cursor + 1) % self.fps_history.len();
    }

    #[cfg(feature = "egui")]
    fn debug_panel(&mut self, ctx: &egui::Context, frame_index: usize) {
        use egui::widgets::Slider;

        if !self.in_editor() {
            return;
        }

        let render_world = &mut self.render_worlds[frame_index % Self::RENDER_FRAME_COUNT];

        egui::Window::new("World")
            .default_open(true)
            .show(ctx, |ui| {
                use crate::game::scenes::world::systems::DebugQuadTreeOptions;

                ui.heading("Environment");
                ui.horizontal(|ui| {
                    ui.label("Time of day");
                    ui.add(
                        Slider::new(&mut self.sim_world.time_of_day, 0.0..=24.0)
                            .drag_value_speed(0.01),
                    );
                });

                // Quad tree
                {
                    ui.heading("Quad Tree");
                    if ui
                        .radio(
                            matches!(
                                self.systems.culling.debug_quad_tree,
                                DebugQuadTreeOptions::None
                            ),
                            "None",
                        )
                        .clicked()
                    {
                        self.systems.culling.debug_quad_tree = DebugQuadTreeOptions::None;
                    }

                    ui.horizontal(|ui| {
                        if ui
                            .radio(
                                matches!(
                                    self.systems.culling.debug_quad_tree,
                                    DebugQuadTreeOptions::Level(_)
                                ),
                                "Level",
                            )
                            .clicked()
                        {
                            self.systems.culling.debug_quad_tree = DebugQuadTreeOptions::Level(0);
                        };
                        if let DebugQuadTreeOptions::Level(level) =
                            &mut self.systems.culling.debug_quad_tree
                        {
                            ui.add(egui::widgets::Slider::new(
                                level,
                                0..=(self.sim_world.quad_tree.max_level),
                            ));
                        }
                    });

                    if ui
                        .radio(
                            matches!(
                                self.systems.culling.debug_quad_tree,
                                DebugQuadTreeOptions::All
                            ),
                            "All",
                        )
                        .clicked()
                    {
                        self.systems.culling.debug_quad_tree = DebugQuadTreeOptions::All;
                    }
                }

                // Terrain
                {
                    ui.heading("Terrain");
                    ui.checkbox(
                        &mut self.systems.terrain_system.debug_render_terrain_wireframe,
                        "Render terrain wireframe",
                    );
                }

                // Objects
                {
                    ui.heading("Objects");
                    ui.checkbox(
                        &mut self.systems.objects_system.debug_render_bounding_spheres,
                        "Render bounding spheres",
                    );
                }

                {
                    ui.add(
                        Slider::new(&mut self.sim_world.timer, 0.0..=240.0).drag_value_speed(0.1),
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
                    egui::Color32::from_rgb(255, 0, 0),
                    egui::Color32::from_rgb(0, 255, 0),
                    egui::Color32::from_rgb(0, 0, 255),
                    egui::Color32::from_rgb(255, 255, 0),
                    egui::Color32::from_rgb(255, 0, 255),
                ];

                for i in 0..self.fps_history.len() {
                    let frame_time =
                        self.fps_history[(self.fps_history_cursor + i) % self.fps_history.len()];

                    let left = rect.left() + i as f32 * bar_width;
                    let right = rect.left() + (i + 1) as f32 * bar_width;

                    let mut bottom = rect.bottom();

                    for (c, value) in [
                        frame_time.input,
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
            ui.horizontal(|ui| {
                ui.label("Frame index");
                ui.label(format!("{frame_index}"));
            });
            ui.horizontal(|ui| {
                ui.label("Visible chunks");
                ui.label(format!("{}", self.sim_world.visible_chunks.len()));
            });
            ui.horizontal(|ui| {
                ui.label("Visible objects");
                ui.label(format!("{}", self.sim_world.visible_objects.len()));
            });
            ui.horizontal(|ui| {
                ui.label("Visible strata");
                ui.label(format!("{}", render_world.strata_instances.len()));
            });
            ui.horizontal(|ui| {
                ui.label("Gizmo vertices");
                ui.label(format!("{}", render_world.gizmo_vertices.len()));
            });
        });

        // self.objects.debug_panel(ctx);
    }
}
