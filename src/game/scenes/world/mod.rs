use crate::{
    engine::prelude::*,
    game::{
        config::CampaignDef,
        data_dir::data_dir,
        math::RaySegment,
        scenes::world::{
            game_mode::GameMode,
            render::{RenderStore, RenderWorld},
            sim_world::SimWorld,
        },
    },
};

pub mod actions;
mod animation;
mod game_mode;
pub mod height_map;
mod objects;
mod quad_tree;
mod render;
mod sim_world;
mod systems;
mod terrain;

/// The [Scene] that renders the ingame world view.
pub struct WorldScene {
    sim_world: SimWorld,
    render_worlds: [RenderWorld; Self::RENDER_FRAME_COUNT],
    render_store: RenderStore,

    // Systems
    systems: systems::Systems,

    game_mode: GameMode,

    last_frame_time: std::time::Instant,
    fps_history: Vec<f32>,
    fps_history_cursor: usize,
}

impl WorldScene {
    const RENDER_FRAME_COUNT: usize = 3;

    pub fn new(campaign_def: CampaignDef, window_size: UVec2) -> Result<Self, AssetError> {
        tracing::info!("Loading campaign \"{}\"...", campaign_def.title);

        let campaign = data_dir().load_campaign(&campaign_def.base_name)?;

        let fps_history = vec![0.0; 100];
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

            last_frame_time: std::time::Instant::now(),
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
        // Run systems
        {
            let time = systems::Time { delta_time };
            let viewport_size = renderer().surface.size();

            self.systems
                .input(&mut self.sim_world, &time, input, viewport_size);
            self.systems.update(&mut self.sim_world, &time);
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

        // Systems
        {
            self.systems
                .extract(&mut self.sim_world, &mut self.render_store, render_world);
            self.systems
                .prepare(&mut self.render_store, render_world, renderer());
            self.systems.queue(&self.render_store, render_world, frame);
        }

        let now = std::time::Instant::now();
        let render_time = now - self.last_frame_time;
        self.last_frame_time = now;

        self.fps_history[self.fps_history_cursor] = render_time.as_secs_f32();
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

                // Objects
                {
                    ui.heading("Objects");
                    ui.checkbox(
                        &mut self.systems.objects_system.debug_render_bounding_spheres,
                        "Render bounding spheres",
                    )
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

                let fps_range = 288.0;

                for i in 0..self.fps_history.len() {
                    let value =
                        self.fps_history[(self.fps_history_cursor + i) % self.fps_history.len()];

                    let left = rect.left() + i as f32 * bar_width;
                    let right = rect.left() + (i + 1) as f32 * bar_width;

                    let bottom = rect.bottom();
                    let top = bottom - rect.height() * ((1.0 / value) / fps_range);

                    let bar_rect = egui::Rect::from_min_max(
                        egui::Pos2::new(left, top),
                        egui::Pos2::new(right, bottom),
                    );
                    painter.rect_filled(bar_rect, 0.0, egui::Color32::RED);
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
