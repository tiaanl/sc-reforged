use std::{collections::HashMap, path::PathBuf};

use crate::{
    engine::{
        gizmos::{GizmoVertex, GizmosRenderer},
        prelude::*,
        shaders::Shaders,
    },
    game::{
        animation::Track,
        asset_loader::{AssetError, AssetLoader},
        camera::{self, Controller},
        config::{self, CampaignDef, LodModelProfileDefinition, SubModelDefinition},
    },
};
use egui::Widget;
use glam::{Quat, Vec3, Vec4, Vec4Swizzles};
use terrain::*;
use wgpu::util::DeviceExt;

mod bounding_boxes;
mod height_map;
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
    _campaign_def: CampaignDef,

    view_debug_camera: bool,
    control_debug_camera: bool,

    main_camera: Camera<camera::GameCameraController>,
    debug_camera: Camera<camera::FreeCameraController>,

    window_size: Vec2,
    intersection: Option<Vec3>,

    terrain: Terrain,
    objects: objects::Objects,

    gizmos_renderer: GizmosRenderer,

    // Input handling.
    under_mouse: UnderMouse,
    selected_object: Option<usize>,

    new_id: String,

    // test
    terrain_height_sample: Vec2,

    lod_model_definitions: HashMap<String, Vec<SubModelDefinition>>,

    time_of_day: f32,
    day_night_cycle: DayNightCycle,

    environment: Environment,

    environment_buffer: wgpu::Buffer,
    environment_bind_group_layout: wgpu::BindGroupLayout,
    environment_bind_group: wgpu::BindGroup,
}

#[derive(Debug)]
enum UnderMouse {
    Nothing { position: Vec2 },
    Object { object_index: usize, position: Vec2 },
}

impl WorldScene {
    pub fn new(
        asset_loader: &AssetLoader,
        asset_store: AssetStore,
        renderer: &Renderer,
        campaign_def: CampaignDef,
    ) -> Result<Self, AssetError> {
        tracing::info!("Loading campaign \"{}\"...", campaign_def.title);

        let lod_model_definitions = {
            let mut lod_definitions: HashMap<String, Vec<SubModelDefinition>> = HashMap::default();

            for lod_path in asset_loader
                .enum_dir(r"config\lod_model_profiles")
                .map_err(|err| {
                    AssetError::FileSystemError(crate::game::vfs::FileSystemError::Io(err))
                })?
                .into_iter()
                .filter(|e| e.as_path().extension().unwrap() == "txt")
            {
                let profile = asset_loader.load_config::<LodModelProfileDefinition>(lod_path)?;
                lod_definitions.insert(profile.lod_model_name, profile.sub_model_definitions);
            }

            lod_definitions
        };

        // Load the campaign specification.
        let campaign = asset_loader.load_config::<config::Campaign>(
            PathBuf::from("campaign")
                .join(&campaign_def.base_name)
                .join(&campaign_def.base_name)
                .with_extension("txt"),
        )?;

        let mut shaders = Shaders::new();
        camera::register_camera_shader(&mut shaders);
        shaders.add_module(include_str!("environment.wgsl"), "environment.wgsl");
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
            asset_loader,
            renderer,
            &mut shaders,
            &campaign_def,
            &main_camera.gpu_camera.bind_group_layout,
            &environment_bind_group_layout,
        )?;

        let mut objects = objects::Objects::new(
            asset_store.clone(),
            renderer,
            &mut shaders,
            &main_camera.gpu_camera.bind_group_layout,
            &environment_bind_group_layout,
        );

        if let Some(mtf_name) = campaign.mtf_name {
            let mtf =
                asset_loader.load_config::<config::Mtf>(PathBuf::from("maps").join(mtf_name))?;

            let mut to_spawn = mtf
                .objects
                .iter()
                .flat_map(|object| {
                    let prefix = {
                        let mut prefix = PathBuf::from("models");
                        if object.typ == "Bipedal" {
                            prefix = prefix.join("people").join("bodies")
                        }
                        prefix
                    };

                    let path = if let Some(defs) = lod_model_definitions.get(&object.name) {
                        let model_name = &defs[0].sub_model_model;

                        prefix
                            .join(model_name)
                            .join(model_name)
                            .with_extension("smf")
                    } else {
                        prefix
                            .join(&object.name)
                            .join(&object.name)
                            .with_extension("smf")
                    };

                    let model_handle = match asset_loader.load_smf(&path, renderer) {
                        Ok(handle) => handle,
                        Err(err) => {
                            tracing::warn!("Could not load .smf model: {}", path.display());
                            return Err(err);
                        }
                    };

                    let object =
                        objects::Object::new(object.position, object.rotation, model_handle);

                    Ok::<_, AssetError>(object)
                })
                .collect::<Vec<_>>();

            for object in to_spawn.drain(..) {
                objects.spawn(object);
            }
        }

        let gizmos_renderer =
            GizmosRenderer::new(renderer, &main_camera.gpu_camera.bind_group_layout);

        Ok(Self {
            _campaign_def: campaign_def,

            view_debug_camera: false,
            control_debug_camera: false,

            main_camera,
            debug_camera,

            window_size: Vec2::ZERO,
            intersection: None,

            terrain,
            objects,

            gizmos_renderer,

            // Input handling.
            under_mouse: UnderMouse::Nothing {
                position: Vec2::ZERO,
            },
            selected_object: None,

            new_id: String::new(),

            terrain_height_sample: Vec2::ZERO,

            lod_model_definitions,

            time_of_day,
            day_night_cycle,
            environment,

            environment_buffer,
            environment_bind_group_layout,
            environment_bind_group,
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
        let width = renderer.surface_config.width;
        let height = renderer.surface_config.height;

        self.window_size = Vec2::new(width as f32, height as f32);
        self.main_camera.camera.aspect_ratio = width as f32 / height.max(1) as f32;
        self.debug_camera.camera.aspect_ratio = width as f32 / height.max(1) as f32;
    }

    fn event(&mut self, event: &SceneEvent) {
        macro_rules! ndc {
            ($position:expr) => {{
                Vec2::new(
                    $position.x / self.window_size.x.max(1.0) * 2.0 - 1.0,
                    (1.0 - $position.y / self.window_size.y.max(1.0)) * 2.0 - 1.0,
                )
            }};
        }

        macro_rules! update_under_mouse {
            ($position:expr,$window_size:expr) => {{
                let ndc = ndc!($position);
                let ray = self.main_camera.camera.generate_ray(ndc);
                if let Some(object_index) = self.objects.ray_intersection(&ray) {
                    UnderMouse::Object {
                        object_index,
                        position: $position,
                    }
                } else {
                    UnderMouse::Nothing {
                        position: $position,
                    }
                }
            }};
        }

        // Update the `under_mouse`.
        match *event {
            SceneEvent::MouseDown {
                position,
                button: MouseButton::Left,
            } => {
                self.under_mouse = update_under_mouse!(position, window_size);
            }
            SceneEvent::MouseMove { position } => {
                self.under_mouse = update_under_mouse!(position, window_size);
            }
            SceneEvent::MouseUp {
                position,
                button: MouseButton::Left,
            } => {
                match self.under_mouse {
                    UnderMouse::Object {
                        object_index: entity_index,
                        position: p,
                    } if (position - p).length_squared() < 100.0 => {
                        // Set the selected entity.
                        self.selected_object = Some(entity_index);
                    }
                    UnderMouse::Nothing { position: p }
                        if (position - p).length_squared() < 100.0 =>
                    {
                        self.selected_object = None;
                    }
                    _ => {}
                }
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

        // Highlight whatever we're hovering on.
        match self.under_mouse {
            UnderMouse::Nothing { .. } => self.objects.set_selected(None),
            UnderMouse::Object {
                object_index: entity_index,
                ..
            } => self.objects.set_selected(Some(entity_index)),
        }

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

        frame.clear_color_and_depth(
            wgpu::Color {
                r: self.environment.fog_color.x as f64,
                g: self.environment.fog_color.y as f64,
                b: self.environment.fog_color.z as f64,
                a: 1.0,
            },
            1.0,
        );

        let camera_bind_group = if self.view_debug_camera {
            &self.debug_camera.gpu_camera.bind_group
        } else {
            &self.main_camera.gpu_camera.bind_group
        };
        let environment_bind_group = &self.environment_bind_group;

        // Render Opaque geometry first.
        self.terrain.render(
            frame,
            camera_bind_group,
            &self.main_camera.gpu_camera.bind_group, // Always the main camera.
            environment_bind_group,
        );
        self.objects
            .render_objects(frame, camera_bind_group, environment_bind_group);

        // Now render alpha geoometry.
        self.terrain
            .render_water(frame, camera_bind_group, environment_bind_group);
        self.objects
            .render_alpha_objects(frame, camera_bind_group, environment_bind_group);

        // Render any kind of debug overlays.
        self.terrain.render_gizmos(
            frame,
            camera_bind_group,
            environment_bind_group,
            &self.gizmos_renderer,
        );
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
    }

    fn debug_panel(&mut self, egui: &egui::Context, _renderer: &Renderer) {
        use egui::widgets::{DragValue, Slider};

        egui::Window::new("World")
            .default_open(false)
            .show(egui, |ui| {
                if let Some(intersection) = self.intersection {
                    ui.label("Intersection");
                    ui.label(format!("{}", intersection));
                }

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

                ui.heading("Entities");
                self.objects.debug_panel(ui);

                ui.heading("Object");
                if let Some(e) = self.selected_object {
                    ui.label(format!("{}", e));
                    if let Some(object) = self.objects.get_mut(e) {
                        ui.horizontal(|ui| {
                            ui.label("translation");
                            DragValue::new(&mut object.translation.x).ui(ui);
                            DragValue::new(&mut object.translation.y).ui(ui);
                            DragValue::new(&mut object.translation.z).ui(ui);
                        });
                        ui.horizontal(|ui| {
                            ui.label("rotation");
                            DragValue::new(&mut object.rotation.x).speed(0.01).ui(ui);
                            DragValue::new(&mut object.rotation.y).speed(0.01).ui(ui);
                            DragValue::new(&mut object.rotation.z).speed(0.01).ui(ui);
                        });
                    }
                } else {
                    ui.label("Nothing");
                }
            });
    }
}
