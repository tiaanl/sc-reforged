use terrain::Terrain;
use wgpu::util::DeviceExt;

use crate::{
    engine::{
        gizmos::{GizmoVertex, GizmosRenderer},
        prelude::*,
    },
    game::{
        animations::track::Track,
        camera::{self, Controller},
        compositor::Compositor,
        config::{CampaignDef, ObjectType},
        data_dir::data_dir,
        geometry_buffers::{GeometryBuffers, GeometryData, RenderTarget},
        physics::Physics,
        scenes::world::{
            actions::PlayerAction, game_mode::GameMode, overlay_renderer::OverlayRenderer,
        },
        shadows::ShadowCascades,
    },
};

pub mod actions;
mod game_mode;
mod object;
mod objects;
mod overlay_renderer;
mod strata;
mod terrain;

#[derive(Default)]
struct DayNightCycle {
    sun_dir: Track<Vec3>,
    sun_color: Track<Vec3>,

    fog_distance: Track<f32>,
    fog_near_fraction: Track<f32>,
    fog_color: Track<Vec3>,
}

#[derive(Clone, Copy, Default, bytemuck::NoUninit)]
#[repr(C)]
struct GpuEnvironment {
    pub sun_dir: [f32; 4],
    pub sun_color: [f32; 4],

    pub fog_color: [f32; 4],
    pub fog_params: [f32; 4],
}

/// Wrap all the data for controlling the camera.
struct Camera<C: camera::Controller> {
    controller: C,
    camera: camera::Camera,
    gpu_camera: camera::GpuCamera,
}

/// The [Scene] that renders the ingame world view.
pub struct WorldScene {
    game_mode: GameMode,

    view_debug_camera: bool,
    control_debug_camera: bool,

    main_camera: Camera<camera::GameCameraController>,
    debug_camera: Camera<camera::FreeCameraController>,

    terrain: Terrain,
    objects: objects::Objects,

    // Render
    shadow_cascades: ShadowCascades,
    shadow_cascades_lambda: f32,
    geometry_buffers: GeometryBuffers,
    compositor: Compositor,
    shadow_render_target: RenderTarget,

    overlay_renderer: OverlayRenderer,

    time_of_day: f32,
    day_night_cycle: DayNightCycle,

    environment: GpuEnvironment,
    ambient_color: Vec3,

    environment_buffer: wgpu::Buffer,
    environment_bind_group: wgpu::BindGroup,

    last_mouse_position: Option<UVec2>,
    geometry_data: Option<GeometryData>,

    last_frame_time: std::time::Instant,
    fps_history: Vec<f32>,
    fps_history_cursor: usize,

    gizmos_vertices: Vec<GizmoVertex>,
    gizmos_renderer: GizmosRenderer,

    render_overlay: bool,

    physics: Physics,
}

impl WorldScene {
    const SHADOW_MAP_RESOLUTION: u32 = 2048;

    pub fn new(campaign_def: CampaignDef) -> Result<Self, AssetError> {
        tracing::info!("Loading campaign \"{}\"...", campaign_def.title);

        let campaign = data_dir().load_campaign(&campaign_def.base_name)?;

        let main_camera = {
            let camera_from = campaign.view_initial.from.extend(2500.0);
            let camera_to = campaign.view_initial.to.extend(0.0);

            let mut controller = camera::GameCameraController::new(1000.0, 0.2);
            controller.move_to_direct(camera_from);
            controller.look_at_direct(camera_to);
            let camera = camera::Camera::new(
                camera_from,
                Quat::IDENTITY,
                45.0_f32.to_radians(),
                1.0,
                10.0,
                13_300.0,
            );
            let gpu_camera = camera::GpuCamera::new(&renderer().device);

            Camera {
                controller,
                camera,
                gpu_camera,
            }
        };

        let debug_camera = {
            let controller = camera::FreeCameraController::new(1000.0, 0.2);
            let camera = camera::Camera::new(
                Vec3::new(0.0, 0.0, 10_000.0),
                Quat::IDENTITY,
                45.0_f32.to_radians(),
                1.0,
                10.0,
                150_000.0,
            );
            let gpu_camera = camera::GpuCamera::new(&renderer().device);

            Camera {
                controller,
                camera,
                gpu_camera,
            }
        };

        let time_of_day = 12.0;
        let day_night_cycle = {
            let mut e = DayNightCycle::default();

            campaign.time_of_day.iter().enumerate().for_each(|(i, t)| {
                let index = i as u32;

                e.sun_dir.insert(index, t.sun_dir);
                e.sun_color.insert(index, t.sun_color);

                e.fog_distance.insert(index, t.fog_distance);
                e.fog_near_fraction.insert(index, t.fog_near_fraction);
                e.fog_color.insert(index, t.fog_color);
            });

            e
        };

        let mut physics = Physics::new();

        let environment = GpuEnvironment::default();
        let ambient_color = Vec3::splat(0.3);

        let environment_buffer =
            renderer()
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("environment_buffer"),
                    contents: bytemuck::cast_slice(&[environment]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let environment_bind_group_layout =
            renderer()
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("environment_bind_group_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX
                            | wgpu::ShaderStages::FRAGMENT
                            | wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let environment_bind_group =
            renderer()
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("environment_bind_group_layout"),
                    layout: &environment_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(
                            environment_buffer.as_entire_buffer_binding(),
                        ),
                    }],
                });

        let shadow_cascades = ShadowCascades::new(&renderer().device, Self::SHADOW_MAP_RESOLUTION);

        let geometry_buffers = GeometryBuffers::new(&renderer().device, renderer().surface.size());

        // Start with a 4K shadow buffer.
        let shadow_render_target = RenderTarget::new(
            &renderer().device,
            "shadow",
            UVec2::new(Self::SHADOW_MAP_RESOLUTION, Self::SHADOW_MAP_RESOLUTION),
            wgpu::TextureFormat::Depth32Float,
        );

        let compositor = Compositor::new(
            &geometry_buffers.bind_group_layout,
            &main_camera.gpu_camera.bind_group_layout,
            &environment_bind_group_layout,
        );

        let terrain = Terrain::new(
            &campaign_def,
            &main_camera.gpu_camera.bind_group_layout,
            &environment_bind_group_layout,
            &shadow_cascades,
            &mut physics,
        )?;

        let mut objects = objects::Objects::new(
            &main_camera.gpu_camera.bind_group_layout,
            &environment_bind_group_layout,
            &shadow_cascades,
        )?;

        if let Some(ref mtf_name) = campaign.mtf_name {
            let mtf = data_dir().load_mtf(mtf_name)?;

            for object in mtf.objects.iter() {
                let object_type = ObjectType::from_string(&object.typ)
                    .unwrap_or_else(|| panic!("missing object type: {}", object.typ));

                if let Err(err) = objects.spawn(
                    // Rotate objects to the left.
                    Transform::from_translation(object.position)
                        .with_euler_rotation(object.rotation * Vec3::new(1.0, 1.0, -1.0)),
                    object_type,
                    &object.name,
                    &object.title,
                    &mut physics,
                ) {
                    tracing::error!("Could not load model: {}", err);
                }
            }
        }

        let overlay_renderer = OverlayRenderer::new(
            &main_camera.gpu_camera.bind_group_layout,
            &shadow_cascades,
            &geometry_buffers,
        );

        let gizmos_renderer = GizmosRenderer::new(&main_camera.gpu_camera.bind_group_layout);

        let fps_history = vec![0.0; 100];
        let fps_history_cursor = 0;

        Ok(Self {
            game_mode: GameMode::Editor,

            view_debug_camera: false,
            control_debug_camera: false,

            main_camera,
            debug_camera,

            terrain,
            objects,

            shadow_cascades,
            shadow_cascades_lambda: 0.5,
            geometry_buffers,
            shadow_render_target,
            compositor,

            overlay_renderer,

            time_of_day,
            day_night_cycle,
            environment,
            ambient_color,

            environment_buffer,
            environment_bind_group,

            last_mouse_position: None,
            geometry_data: None,

            last_frame_time: std::time::Instant::now(),
            fps_history,
            fps_history_cursor,

            gizmos_vertices: Vec::with_capacity(128),
            gizmos_renderer,

            render_overlay: false,

            physics,
        })
    }

    pub fn in_editor(&self) -> bool {
        matches!(self.game_mode, GameMode::Editor)
    }

    fn calculate_environment(&self, time_of_day: f32) -> GpuEnvironment {
        let sun_dir = self
            .day_night_cycle
            .sun_dir
            .sample_sub_frame(time_of_day, true);
        let sun_color = self
            .day_night_cycle
            .sun_color
            .sample_sub_frame(time_of_day, true);

        let fog_far = self
            .day_night_cycle
            .fog_distance
            .sample_sub_frame(time_of_day, true);
        let fog_near = fog_far
            * self
                .day_night_cycle
                .fog_near_fraction
                .sample_sub_frame(time_of_day, true);
        let fog_color = self
            .day_night_cycle
            .fog_color
            .sample_sub_frame(time_of_day, true);

        GpuEnvironment {
            sun_dir: sun_dir.extend(self.ambient_color.x).to_array(),
            sun_color: sun_color.extend(self.ambient_color.y).to_array(),
            fog_color: fog_color.extend(self.ambient_color.z).to_array(),
            fog_params: [fog_near, fog_far, 0.0, 0.0],
        }
    }
}

impl Scene for WorldScene {
    fn resize(&mut self) {
        let size = renderer().surface.size();

        // Replace the buffers with new ones.
        self.geometry_buffers = GeometryBuffers::new(&renderer().device, size);

        let [width, height] = size.to_array().map(|f| f as f32);
        let aspect = width / height.max(1.0);
        self.main_camera.camera.aspect_ratio = aspect;
        self.debug_camera.camera.aspect_ratio = aspect;
    }

    fn event(&mut self, event: SceneEvent) {
        match event {
            SceneEvent::MouseLeft => self.last_mouse_position = None,
            SceneEvent::MouseMove { position } => {
                self.last_mouse_position = Some(position);
            }
            _ => {}
        }
    }

    fn update(&mut self, delta_time: f32, input: &InputState) {
        if input.key_just_pressed(KeyCode::Backquote) {
            self.game_mode = if self.in_editor() {
                GameMode::Game
            } else {
                GameMode::Editor
            }
        }

        if input.mouse_just_pressed(MouseButton::Left) {
            if let Some(ref data) = self.geometry_data {
                // TODO: This needs a better place.
                const TERRAIN_ENTITY_ID: u32 = 1 << 16;

                // Figure out the type of object we clicked on:
                let player_action = if data.id >= TERRAIN_ENTITY_ID {
                    PlayerAction::Terrain {
                        position: data.position,
                    }
                } else {
                    PlayerAction::Object {
                        _position: data.position,
                        id: data.id,
                    }
                };

                self.objects.handle_player_action(&player_action);
            }
        }

        // Advance time of day.
        // self.time_of_day = (self.time_of_day + delta_time * 0.01).rem_euclid(24.0);
        self.environment = self.calculate_environment(self.time_of_day);

        // Set the camera far plane to max distance of the fog.
        self.main_camera.camera.far = self.environment.fog_params[1];
        if self.control_debug_camera {
            self.debug_camera.controller.update(delta_time, input);
        } else {
            self.main_camera.controller.update(delta_time, input);
        }

        self.main_camera
            .controller
            .update_camera(&mut self.main_camera.camera);
        self.debug_camera
            .controller
            .update_camera(&mut self.debug_camera.camera);

        self.physics.step();

        self.objects.update(delta_time, &self.physics);
    }

    fn render(&mut self, frame: &mut Frame) {
        let main_view_projection = {
            let view_projection = self.main_camera.camera.calculate_view_projection();
            let position = self.main_camera.camera.position;

            self.main_camera
                .gpu_camera
                .upload(&view_projection, position);

            view_projection
        };

        let main_camera_frustum = main_view_projection.frustum();

        let _debug_view_projection = {
            let view_projection = self.debug_camera.camera.calculate_view_projection();
            let position = self.debug_camera.camera.position;

            self.debug_camera
                .gpu_camera
                .upload(&view_projection, position);

            view_projection
        };

        let sun_dir = Vec4::from(self.environment.sun_dir).truncate();
        self.shadow_cascades.update_from_camera(
            &self.main_camera.camera,
            sun_dir,
            self.shadow_cascades_lambda,
        );

        renderer().queue.write_buffer(
            &self.environment_buffer,
            0,
            bytemuck::cast_slice(&[self.environment]),
        );

        // Clear the buffers.
        {
            const INVALID_ID: u32 = 0xFFFF_FFFF;
            let positions_clear_color = wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: f32::from_le_bytes(INVALID_ID.to_le_bytes()) as f64,
            };
            let fog_clear_color = wgpu::Color {
                r: self.environment.fog_color[0] as f64,
                g: self.environment.fog_color[1] as f64,
                b: self.environment.fog_color[2] as f64,
                a: 1.0,
            };

            let oit_accumulation_clear = wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            };
            let oit_revealage_clear = wgpu::Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            };

            drop(
                frame
                    .encoder
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("world_clear_render_pass"),
                        color_attachments: &[
                            // Clear the color buffer with the fog color.
                            Some(wgpu::RenderPassColorAttachment {
                                view: &self.geometry_buffers.color.view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(fog_clear_color),
                                    store: wgpu::StoreOp::Store,
                                },
                            }),
                            Some(wgpu::RenderPassColorAttachment {
                                view: &self.geometry_buffers.oit_accumulation.view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(oit_accumulation_clear),
                                    store: wgpu::StoreOp::Store,
                                },
                            }),
                            Some(wgpu::RenderPassColorAttachment {
                                view: &self.geometry_buffers.oit_revealage.view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(oit_revealage_clear),
                                    store: wgpu::StoreOp::Store,
                                },
                            }),
                            // Set positions to 0 and ID's to invalid.
                            Some(wgpu::RenderPassColorAttachment {
                                view: &self.geometry_buffers.position_id.view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(positions_clear_color),
                                    store: wgpu::StoreOp::Store,
                                },
                            }),
                        ],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &self.geometry_buffers.depth.view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    }),
            );

            drop(
                frame
                    .encoder
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("light_clear_render_pass"),
                        color_attachments: &[],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &self.shadow_render_target.view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    }),
            );

            self.shadow_cascades.clear_buffers(&mut frame.encoder);
        }

        // --- Shadow pass ---

        let _z = tracy_client::span!("render shadow map");

        self.objects
            .render_shadow_casters(frame, &self.shadow_cascades);

        // --- Color pass ---

        let view_camera_bind_group = if self.in_editor() && self.view_debug_camera {
            &self.debug_camera.gpu_camera.bind_group
        } else {
            &self.main_camera.gpu_camera.bind_group
        };

        {
            // Render Opaque geometry first.
            self.terrain.render(
                frame,
                self.in_editor(),
                &self.geometry_buffers,
                &self.shadow_cascades,
                view_camera_bind_group,
                &self.environment_bind_group,
                &self.main_camera.gpu_camera.bind_group, // Always the main camera.
            );

            self.objects.render_objects(
                frame,
                &main_camera_frustum,
                &self.geometry_buffers,
                view_camera_bind_group,
                &self.environment_bind_group,
                &self.shadow_cascades,
            );

            // Now render alpha geoometry.
            self.terrain.render_water(
                frame,
                &self.geometry_buffers,
                view_camera_bind_group,
                &self.environment_bind_group,
            );
        }

        if self.in_editor() && self.render_overlay {
            self.overlay_renderer.render(
                frame,
                &self.main_camera.gpu_camera.bind_group,
                &self.shadow_cascades,
                &self.geometry_buffers,
            );
        } else {
            self.compositor.render(
                frame,
                &self.geometry_buffers,
                &self.main_camera.gpu_camera.bind_group,
                &self.environment_bind_group,
            );
        }

        if self.in_editor() {
            let _z = tracy_client::span!("render gizmos");

            self.gizmos_vertices.clear();

            // Render any kind of debug overlays.
            self.terrain.render_gizmos(&mut self.gizmos_vertices);
            self.objects.render_gizmos(&mut self.gizmos_vertices);

            // Render the main camera frustum when we're looking through the debug camera.
            if self.view_debug_camera {
                self.gizmos_vertices
                    .extend(GizmosRenderer::create_view_projection(
                        &main_view_projection,
                        Vec4::new(1.0, 0.0, 1.0, 1.0),
                    ));
            }

            if let Some(ref data) = self.geometry_data {
                // Translation matrix
                let translation = Mat4::from_translation(data.position);

                let vertices = GizmosRenderer::create_axis(translation, 100.0);
                self.gizmos_renderer
                    .render(frame, view_camera_bind_group, &vertices);
            }

            // Render out the physics data.
            // self.physics.debug_render(&mut self.gizmos_vertices);

            self.gizmos_renderer
                .render(frame, view_camera_bind_group, &self.gizmos_vertices);
        }

        let now = std::time::Instant::now();
        let render_time = now - self.last_frame_time;
        self.last_frame_time = now;

        self.fps_history[self.fps_history_cursor] = render_time.as_secs_f32();
        self.fps_history_cursor = (self.fps_history_cursor + 1) % self.fps_history.len();
    }

    fn post_render(&mut self) {
        self.geometry_data = self.last_mouse_position.map(|position| {
            self.geometry_buffers
                .fetch_data(&renderer().device, &renderer().queue, position)
        });
    }

    #[cfg(feature = "egui")]
    fn debug_panel(&mut self, ctx: &egui::Context) {
        use egui::widgets::Slider;

        if !self.in_editor() {
            return;
        }

        egui::Window::new("World")
            .default_open(true)
            .show(ctx, |ui| {
                ui.heading("Render");
                ui.checkbox(&mut self.render_overlay, "Render overlay");

                ui.heading("Camera");
                ui.checkbox(&mut self.view_debug_camera, "View debug camera");
                ui.checkbox(&mut self.control_debug_camera, "Control debug camera");

                ui.heading("Environment");
                ui.horizontal(|ui| {
                    ui.label("Time of day");
                    ui.add(Slider::new(&mut self.time_of_day, 0.0..=24.0).drag_value_speed(0.01));
                });

                ui.add(Slider::new(&mut self.shadow_cascades_lambda, 0.0..=1.0));
                ui.collapsing("Ambient", |ui| {
                    ui.add(Slider::new(&mut self.ambient_color.x, 0.0..=1.0));
                    ui.add(Slider::new(&mut self.ambient_color.y, 0.0..=1.0));
                    ui.add(Slider::new(&mut self.ambient_color.z, 0.0..=1.0));
                });

                ui.heading("Terrain");
                self.terrain.debug_panel(ui);

                ui.heading("Geometry Data");
                if let Some(ref geometry_data) = self.geometry_data {
                    ui.label(format!(
                        "position: {:.2}, {:.2}, {:.2}",
                        geometry_data.position.x,
                        geometry_data.position.y,
                        geometry_data.position.z,
                    ));
                    ui.label(format!("id: {:?}", geometry_data.id));
                } else {
                    ui.label("None");
                }

                ui.heading("Entities");
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

        self.objects.debug_panel(ctx);
    }
}
