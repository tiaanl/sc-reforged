use ahash::HashMap;
use glam::Vec4Swizzles;
use terrain::Terrain;
use wgpu::util::DeviceExt;

use crate::{
    engine::{
        gizmos::{GizmoVertex, GizmosRenderer},
        prelude::*,
    },
    game::{
        animation::Track,
        camera::{self, Controller},
        compositor::Compositor,
        config::{CampaignDef, SubModelDefinition},
        data_dir::DataDir,
        geometry_buffers::{GeometryBuffers, GeometryData},
    },
};

mod bounding_boxes;
mod objects;
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
struct Environment {
    pub sun_dir: Vec4,
    pub sun_color: Vec4,

    pub fog_color: Vec4,
    pub fog_params: Vec4,
}

/// Wrap all the data for controlling the camera.
struct Camera<C: camera::Controller> {
    controller: C,
    camera: camera::Camera,
    matrices: Tracked<camera::Matrices>,
    gpu_camera: camera::GpuCamera,
}

impl<C: camera::Controller> Camera<C> {
    fn new(renderer: &Renderer, controller: C) -> Self {
        Self {
            controller,
            camera: camera::Camera::default(),
            matrices: Tracked::new(camera::Matrices::default()),
            gpu_camera: camera::GpuCamera::new(renderer),
        }
    }
}

/// The [Scene] that renders the ingame world view.
pub struct WorldScene {
    campaign_def: CampaignDef,

    view_debug_camera: bool,
    control_debug_camera: bool,

    main_camera: Camera<camera::GameCameraController>,
    debug_camera: Camera<camera::FreeCameraController>,

    window_size: Vec2,

    terrain: Terrain,
    objects: objects::Objects,

    // Render
    geometry_buffers: GeometryBuffers,
    compositor: Compositor,
    gizmos_renderer: GizmosRenderer,

    lod_model_definitions: HashMap<String, Vec<SubModelDefinition>>,

    time_of_day: f32,
    day_night_cycle: DayNightCycle,

    environment: Environment,

    environment_buffer: wgpu::Buffer,
    environment_bind_group_layout: wgpu::BindGroupLayout,
    environment_bind_group: wgpu::BindGroup,

    last_mouse_position: Option<UVec2>,
    geometry_data: Option<GeometryData>,
}

#[derive(Debug)]
enum UnderMouse {
    Nothing { position: Vec2 },
    Object { object_index: usize, position: Vec2 },
}

impl WorldScene {
    pub fn new(renderer: &Renderer, campaign_def: CampaignDef) -> Result<Self, AssetError> {
        tracing::info!("Loading campaign \"{}\"...", campaign_def.title);

        let lod_model_definitions = DataDir::load_lod_model_profiles()?;

        let campaign = DataDir::load_campaign(&campaign_def.base_name)?;

        let mut shaders = Shaders::new();
        camera::register_camera_shader(&mut shaders);
        shaders.add_module(include_str!("environment.wgsl"), "environment.wgsl");
        shaders.add_module(
            include_str!("../../common/geometry_buffers.wgsl"),
            "geometry_buffers.wgsl",
        );
        shaders.add_module(include_str!("frustum.wgsl"), "frustum.wgsl");

        let main_camera = {
            let camera_from = campaign.view_initial.from.extend(2500.0);
            let camera_to = campaign.view_initial.to.extend(0.0);

            let mut controller = camera::GameCameraController::new(50.0, 0.2);
            controller.move_to_direct(camera_from);
            controller.look_at_direct(camera_to);
            let camera = camera::Camera::new(
                camera_from,
                Quat::IDENTITY,
                45.0_f32.to_radians(),
                1.0,
                100.0,
                10_000.0,
            );
            let matrices = camera.calculate_matrices().into();
            let gpu_camera = camera::GpuCamera::new(renderer);

            Camera {
                controller,
                camera,
                matrices,
                gpu_camera,
            }
        };

        let debug_camera = {
            let controller = camera::FreeCameraController::new(50.0, 0.2);
            let camera = camera::Camera::new(
                Vec3::new(0.0, 0.0, 10_000.0),
                Quat::IDENTITY,
                45.0_f32.to_radians(),
                1.0,
                100.0,
                150_000.0,
            );
            let matrices = camera.calculate_matrices().into();
            let gpu_camera = camera::GpuCamera::new(renderer);

            Camera {
                controller,
                camera,
                matrices,
                gpu_camera,
            }
        };

        let time_of_day = 12.0;
        let day_night_cycle = {
            let mut e = DayNightCycle::default();

            campaign.time_of_day.iter().enumerate().for_each(|(i, t)| {
                let time = i as f32;
                e.sun_dir.set_key_frame(time, t.sun_dir);
                e.sun_color.set_key_frame(time, t.sun_color);

                e.fog_distance.set_key_frame(time, t.fog_distance);
                e.fog_near_fraction.set_key_frame(time, t.fog_near_fraction);
                e.fog_color.set_key_frame(time, t.fog_color);
            });

            e
        };

        let environment = Environment::default();

        let environment_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("environment_buffer"),
                    contents: bytemuck::cast_slice(&[environment]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let environment_bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("environment_bind_group_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let environment_bind_group =
            renderer
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

        let terrain = Terrain::new(
            renderer,
            &mut shaders,
            &campaign_def,
            &main_camera.gpu_camera.bind_group_layout,
        )?;

        let mut objects = objects::Objects::new(
            renderer,
            &mut shaders,
            &main_camera.gpu_camera.bind_group_layout,
        );

        if let Some(ref mtf_name) = campaign.mtf_name {
            let mtf = DataDir::load_mtf(mtf_name)?;

            for object in mtf.objects.iter() {
                if let Err(err) = objects.spawn(
                    renderer,
                    object.position,
                    Vec3::new(object.rotation.x, object.rotation.y, -object.rotation.z),
                    &object.name,
                ) {
                    tracing::error!("Could not load model: {}", err);
                }
            }
        }

        let geometry_buffers = GeometryBuffers::new(renderer);
        let compositor = Compositor::new(
            renderer,
            &mut shaders,
            &geometry_buffers.bind_group_layout,
            &main_camera.gpu_camera.bind_group_layout,
            &environment_bind_group_layout,
        );

        let gizmos_renderer =
            GizmosRenderer::new(renderer, &main_camera.gpu_camera.bind_group_layout);

        Ok(Self {
            campaign_def,

            view_debug_camera: false,
            control_debug_camera: false,

            main_camera,
            debug_camera,

            window_size: Vec2::ZERO,

            terrain,
            objects,

            geometry_buffers,
            compositor,
            gizmos_renderer,

            lod_model_definitions,

            time_of_day,
            day_night_cycle,
            environment,

            environment_buffer,
            environment_bind_group_layout,
            environment_bind_group,

            last_mouse_position: None,
            geometry_data: None,
        })
    }

    fn calculate_environment(&self, time_of_day: f32) -> Environment {
        let sun_dir = self.day_night_cycle.sun_dir.get(time_of_day);
        let sun_color = self.day_night_cycle.sun_color.get(time_of_day);

        let fog_far = self.day_night_cycle.fog_distance.get(time_of_day);
        let fog_near = fog_far * self.day_night_cycle.fog_near_fraction.get(time_of_day);
        let fog_color = self.day_night_cycle.fog_color.get(time_of_day);

        Environment {
            sun_dir: sun_dir.extend(0.0),
            sun_color: sun_color.extend(0.0),
            fog_color: fog_color.extend(0.0),
            fog_params: Vec4::new(fog_near, fog_far, 0.0, 0.0),
        }
    }
}

impl Scene for WorldScene {
    fn resize(&mut self, renderer: &Renderer) {
        // Replace the buffers with new ones.
        self.geometry_buffers = GeometryBuffers::new(renderer);

        let width = renderer.surface_config.width;
        let height = renderer.surface_config.height;

        self.window_size = Vec2::new(width as f32, height as f32);
        self.main_camera.camera.aspect_ratio = width as f32 / height.max(1) as f32;
        self.debug_camera.camera.aspect_ratio = width as f32 / height.max(1) as f32;
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

    fn update(&mut self, _renderer: &Renderer, delta_time: f32, input: &InputState) {
        const GIZMO_SCALE: f32 = 1000.0;
        const CENTER: Vec3 = Vec3::ZERO;
        const RED: Vec4 = Vec4::new(1.0, 0.0, 0.0, 1.0);
        const GREEN: Vec4 = Vec4::new(0.0, 1.0, 0.0, 1.0);
        const BLUE: Vec4 = Vec4::new(0.0, 0.0, 1.0, 1.0);

        self.time_of_day = (self.time_of_day + delta_time * 0.01).rem_euclid(24.0);
        self.environment = self.calculate_environment(self.time_of_day);

        // Set the camera far plane to the `max_view_distance`.
        self.main_camera.camera.far = self.terrain.max_view_distance;
        if self.control_debug_camera {
            self.debug_camera.controller.update(delta_time, input);
        } else {
            self.main_camera.controller.update(delta_time, input);
        }

        self.main_camera
            .controller
            .update_camera_if_dirty(&mut self.main_camera.camera);
        self.debug_camera
            .controller
            .update_camera_if_dirty(&mut self.debug_camera.camera);

        self.objects.update(&self.main_camera.camera);
    }

    fn render(&mut self, frame: &mut Frame) {
        {
            let matrices = self.main_camera.camera.calculate_matrices();
            let position = self.main_camera.camera.position;
            self.main_camera
                .gpu_camera
                .upload_matrices(&frame.queue, &matrices, position);
        }

        {
            let matrices = self.debug_camera.camera.calculate_matrices();
            let position = self.debug_camera.camera.position;
            self.debug_camera
                .gpu_camera
                .upload_matrices(&frame.queue, &matrices, position);
        }

        frame.queue.write_buffer(
            &self.environment_buffer,
            0,
            bytemuck::cast_slice(&[self.environment]),
        );

        // Clear the buffers.
        {
            const CLEAR_VALUE: u32 = u32::MAX;
            frame
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("world_clear_render_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.geometry_buffers.ids.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: CLEAR_VALUE as f64,
                                g: 0.0,
                                b: 0.0,
                                a: 0.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &frame.depth_buffer.texture_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
        }

        let camera_bind_group = if self.view_debug_camera {
            &self.debug_camera.gpu_camera.bind_group
        } else {
            &self.main_camera.gpu_camera.bind_group
        };

        // Render Opaque geometry first.
        self.terrain.render(
            frame,
            &self.geometry_buffers,
            camera_bind_group,
            &self.main_camera.gpu_camera.bind_group, // Always the main camera.
        );
        self.objects
            .render_objects(frame, &self.geometry_buffers, camera_bind_group);

        // Now render alpha geoometry.
        self.terrain.render_water(frame, camera_bind_group);
        // self.objects
        //     .render_alpha_objects(frame, &self.geometry_buffers, camera_bind_group);

        // Render any kind of debug overlays.
        self.terrain
            .render_gizmos(frame, camera_bind_group, &self.gizmos_renderer);
        self.objects
            .render_gizmos(frame, camera_bind_group, &self.gizmos_renderer);

        if false {
            // Render the direction of the sun.
            let vertices = [
                GizmoVertex::new(Vec3::ZERO, Vec4::new(0.0, 0.0, 1.0, 1.0)),
                GizmoVertex::new(
                    self.environment.sun_dir.xyz() * 1_000.0,
                    Vec4::new(0.0, 0.0, 1.0, 1.0),
                ),
            ];
            self.gizmos_renderer
                .render(frame, camera_bind_group, &vertices);
        }

        // Render the main camera frustum when we're looking through the debug camera.
        if self.view_debug_camera {
            let mut v = vec![];
            camera::render_camera_frustum(&self.main_camera.camera, &mut v);
            self.gizmos_renderer.render(frame, camera_bind_group, &v);
        }

        self.compositor.render(
            frame,
            &self.geometry_buffers,
            &self.main_camera.gpu_camera.bind_group,
            &self.environment_bind_group,
        );

        self.geometry_data = self.last_mouse_position.map(|position| {
            self.geometry_buffers
                .fetch_data(&frame.device, &frame.queue, position)
        });

        if let Some(ref data) = self.geometry_data {
            let up = data.normal;

            // Define a "forward" vector that lies along the surface
            // We'll construct an orthonormal basis (right, up, forward)
            let forward_hint = if data.normal.z.abs() < 0.99 {
                Vec3::Z
            } else {
                Vec3::X
            };

            // Right = up × forward
            let right = up.cross(forward_hint).normalize();
            // Recomputed forward = right × up
            let forward = right.cross(up).normalize();

            // Rotation matrix from basis
            let rotation = Mat4::from_cols(
                right.extend(0.0),
                up.extend(0.0),
                forward.extend(0.0),
                Vec4::W, // placeholder
            );

            // Translation matrix
            let translation = Mat4::from_translation(data.position);

            let vertices = GizmosRenderer::_create_axis(translation * rotation, 100.0);
            self.gizmos_renderer
                .render(frame, camera_bind_group, &vertices);
        }
    }

    #[cfg(feature = "egui")]
    fn debug_panel(&mut self, egui: &egui::Context, _renderer: &Renderer) {
        use egui::widgets::Slider;

        egui::Window::new("World")
            .default_open(false)
            .show(egui, |ui| {
                ui.heading("Camera");
                ui.checkbox(&mut self.view_debug_camera, "View debug camera");
                ui.checkbox(&mut self.control_debug_camera, "Control debug camera");

                ui.heading("Environment");
                ui.horizontal(|ui| {
                    ui.label("Time of day");
                    ui.add(Slider::new(&mut self.time_of_day, 0.0..=24.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Sun dir");
                    ui.label(format!(
                        "{:.2}, {:.2}, {:.2}",
                        self.environment.sun_dir.x,
                        self.environment.sun_dir.y,
                        self.environment.sun_dir.z,
                    ));
                });

                ui.heading("Terrain");
                self.terrain.debug_panel(ui);

                ui.heading("Geometry Data");
                if let Some(ref geometry_data) = self.geometry_data {
                    ui.label(format!("color: {:?}", geometry_data.color));
                    ui.label(format!("position: {:?}", geometry_data.position));
                    ui.label(format!("normal: {:?}", geometry_data.normal));
                    ui.label(format!("id: {:?}", geometry_data.id));
                } else {
                    ui.label("None");
                }

                ui.heading("Entities");
                self.objects.debug_panel(ui);
            });
    }
}
