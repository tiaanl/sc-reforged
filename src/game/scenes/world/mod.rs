use std::{collections::HashMap, path::PathBuf};

use crate::{
    engine::{
        gizmos::{GizmoVertex, GizmosRenderer},
        prelude::*,
        shaders::Shaders,
    },
    game::{
        asset_loader::{AssetError, AssetLoader},
        camera::{self, render_camera_frustum},
        compositor::Compositor,
        config::{self, CampaignDef, Fog, LodModelProfileDefinition, SubModelDefinition},
    },
};
use egui::Widget;
use glam::{Quat, UVec2, Vec3, Vec4};
use terrain::*;

mod bounding_boxes;
mod fog;
mod height_map;
mod objects;
mod terrain;
mod water;

/// The [Scene] that renders the ingame world view.
pub struct WorldScene {
    asset_store: AssetStore,
    _campaign_def: CampaignDef,

    compositor: Compositor,

    view_debug_camera: bool,
    control_debug_camera: bool,

    camera_controller: camera::GameCameraController,
    camera: camera::Camera,
    debug_camera_controller: camera::FreeCameraController,
    debug_camera: camera::Camera,

    camera_matrices: Tracked<camera::Matrices>,
    gpu_camera: camera::GpuCamera,

    window_size: Vec2,
    intersection: Option<Vec3>,

    terrain: Terrain,
    objects: objects::Objects,

    fog: Option<Fog>,
    gpu_fog: fog::GpuFog,

    gizmos_renderer: GizmosRenderer,
    gizmos_vertices: Vec<GizmoVertex>,

    // Input handling.
    under_mouse: UnderMouse,
    selected_object: Option<usize>,

    new_id: String,

    // test
    terrain_height_sample: Vec2,

    lod_model_definitions: HashMap<String, Vec<SubModelDefinition>>,

    fog_density: f32,
}

#[derive(Debug)]
enum UnderMouse {
    Nothing { position: Vec2 },
    Object { object_index: usize, position: Vec2 },
}

impl WorldScene {
    pub fn new(
        assets: &AssetLoader,
        asset_store: AssetStore,
        renderer: &Renderer,
        campaign_def: CampaignDef,
    ) -> Result<Self, AssetError> {
        tracing::info!("Loading campaign \"{}\"...", campaign_def.title);

        let lod_model_definitions = {
            let mut lod_definitions: HashMap<String, Vec<SubModelDefinition>> = HashMap::default();

            for lod_path in assets
                .enum_dir(r"config\lod_model_profiles")
                .map_err(|err| {
                    AssetError::FileSystemError(crate::game::vfs::FileSystemError::Io(err))
                })?
                .into_iter()
                .filter(|e| e.as_path().extension().unwrap() == "txt")
            {
                let profile = assets.load_config::<LodModelProfileDefinition>(lod_path)?;
                lod_definitions.insert(profile.lod_model_name, profile.sub_model_definitions);
            }

            lod_definitions
        };

        // Load the campaign specification.
        let campaign = assets.load_config::<config::Campaign>(
            PathBuf::from("campaign")
                .join(&campaign_def.base_name)
                .join(&campaign_def.base_name)
                .with_extension("txt"),
        )?;

        let mut shaders = Shaders::default();
        camera::register_camera_shader(&mut shaders);
        shaders.add_module(include_str!("fog.wgsl"), "fog.wgsl");

        let camera_from = campaign.view_initial.from.extend(2500.0);
        let camera_to = campaign.view_initial.to.extend(0.0);

        let mut camera_controller = camera::GameCameraController::new(50.0, 0.2);
        camera_controller.move_to_direct(camera_from);
        camera_controller.look_at_direct(camera_to);
        let camera = camera::Camera::new(
            camera_from,
            Quat::IDENTITY,
            45.0_f32.to_radians(),
            1.0,
            100.0,
            10_000.0,
        );

        let mut debug_camera_controller = camera::FreeCameraController::new(50.0, 0.2);
        debug_camera_controller.move_to(Vec3::new(-5_000.0, -5_000.0, 10_000.0));
        debug_camera_controller.look_at(Vec3::new(10_000.0, 10_000.0, 0.0));
        let debug_camera = camera::Camera::new(
            Vec3::ZERO,
            Quat::IDENTITY,
            45.0_f32.to_radians(),
            1.0,
            100.0,
            100_000.0,
        );

        let camera_matrices = camera.calculate_matrices().into();
        let gpu_camera = camera::GpuCamera::new(renderer);

        let gpu_fog = fog::GpuFog::new(renderer);

        let compositor = Compositor::new(
            renderer.device.clone(),
            UVec2::new(
                renderer.surface_config.width,
                renderer.surface_config.height,
            ),
            renderer.surface_config.format,
            &gpu_fog.bind_group_layout,
        );

        let terrain = Terrain::new(
            assets,
            renderer,
            &mut shaders,
            &campaign_def,
            &gpu_camera.bind_group_layout,
        )?;

        let mut objects = objects::Objects::new(
            asset_store.clone(),
            renderer,
            &mut shaders,
            &gpu_camera.bind_group_layout,
        );

        let mut fog = None;

        if let Some(mtf_name) = campaign.mtf_name {
            let mtf = assets.load_config::<config::Mtf>(PathBuf::from("maps").join(mtf_name))?;

            fog = Some(mtf.game_config_fog_enabled);

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

                    let model_handle = match assets.load_smf(&path, renderer) {
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

        let gizmos_renderer = GizmosRenderer::new(renderer, &gpu_camera.bind_group_layout);

        Ok(Self {
            asset_store,
            _campaign_def: campaign_def,

            compositor,

            view_debug_camera: false,
            control_debug_camera: false,

            camera_controller,
            camera,

            debug_camera_controller,
            debug_camera,

            camera_matrices,
            gpu_camera,

            window_size: Vec2::ZERO,
            intersection: None,

            terrain,
            objects,
            fog,
            gpu_fog,

            gizmos_renderer,
            gizmos_vertices: vec![],

            // Input handling.
            under_mouse: UnderMouse::Nothing {
                position: Vec2::ZERO,
            },
            selected_object: None,

            new_id: String::new(),

            terrain_height_sample: Vec2::ZERO,

            lod_model_definitions,

            fog_density: 1.0,
        })
    }
}

impl Scene for WorldScene {
    fn resize(&mut self, renderer: &Renderer) {
        let width = renderer.surface_config.width;
        let height = renderer.surface_config.height;

        self.compositor.resize(width, height);

        self.window_size = Vec2::new(width as f32, height as f32);
        self.camera.aspect_ratio = width as f32 / height.max(1) as f32;
        self.debug_camera.aspect_ratio = width as f32 / height.max(1) as f32;
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
                let ray = self.camera.generate_ray(ndc);
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

    fn update(&mut self, delta_time: f32, input: &InputState) {
        const GIZMO_SCALE: f32 = 1000.0;
        const CENTER: Vec3 = Vec3::ZERO;
        const RED: Vec4 = Vec4::new(1.0, 0.0, 0.0, 1.0);
        const GREEN: Vec4 = Vec4::new(0.0, 1.0, 0.0, 1.0);
        const BLUE: Vec4 = Vec4::new(0.0, 0.0, 1.0, 1.0);

        // Set the camera far plane to the `max_view_distance`.
        self.camera.far = self.terrain.max_view_distance;

        if self.control_debug_camera {
            self.debug_camera_controller.update(input, delta_time);
        } else {
            self.camera_controller.update(input, delta_time);
        }

        self.debug_camera_controller
            .update_camera_if_dirty(&mut self.debug_camera);
        if self
            .camera_controller
            .update_camera_if_dirty(&mut self.camera)
        {
            *self.camera_matrices = self.camera.calculate_matrices();
        }

        // Highlight whatever we're hovering on.
        match self.under_mouse {
            UnderMouse::Nothing { .. } => self.objects.set_selected(None),
            UnderMouse::Object {
                object_index: entity_index,
                ..
            } => self.objects.set_selected(Some(entity_index)),
        }

        self.terrain.update(&self.camera);

        // Only render the camera frustum if we're looking through the debug camera.
        if self.view_debug_camera {
            render_camera_frustum(&self.camera, &mut self.gizmos_vertices);
        }

        self.objects.update(&self.camera);

        // let height = self.terrain.height_at(self.terrain_height_sample);
        // let pos = self.terrain_height_sample.extend(height);
        // self.gizmos_vertices.extend(GizmosRenderer::_create_axis(
        //     Mat4::from_translation(pos),
        //     100.0,
        // ));
    }

    fn begin_frame(&mut self, _device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.view_debug_camera {
            self.gpu_camera.upload_matrices(
                queue,
                &self.debug_camera.calculate_matrices(),
                self.debug_camera.position,
            );
        } else {
            self.gpu_camera.upload_matrices(
                queue,
                &self.camera.calculate_matrices(),
                self.camera.position,
            );
        }

        if let Some(fog) = &self.fog {
            self.gpu_fog.upload(queue, fog, self.fog_density);
        }
    }

    fn render_frame(&self, frame: &mut Frame) {
        if let Some(fog) = &self.fog {
            frame
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("clear_compositor_layers"),
                    color_attachments: &[
                        Some(wgpu::RenderPassColorAttachment {
                            view: &self.compositor.albedo_texture,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: fog.color.x as f64,
                                    g: fog.color.y as f64,
                                    b: fog.color.z as f64,
                                    a: 1.0,
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                        }),
                        Some(wgpu::RenderPassColorAttachment {
                            view: &self.compositor.position_texture,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.0,
                                    g: 0.0,
                                    b: 0.0,
                                    a: 0.0,
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                        }),
                    ],
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
        } else {
            frame.clear_color_and_depth(
                wgpu::Color {
                    r: 0.1,
                    g: 0.2,
                    b: 0.3,
                    a: 1.0,
                },
                1.0,
            );
        }

        let camera_bind_group = &self.gpu_camera.bind_group;

        self.terrain
            .render(frame, &self.compositor, camera_bind_group);

        self.objects
            .render_objects(frame, &self.compositor, camera_bind_group);

        self.gizmos_renderer
            .render(frame, camera_bind_group, &self.gizmos_vertices);

        self.terrain.render_water(frame, camera_bind_group);

        self.compositor
            .composite(&mut frame.encoder, &frame.surface, &self.gpu_fog.bind_group);
    }

    fn end_frame(&mut self) {
        self.gizmos_vertices.clear();
    }

    fn debug_panel(&mut self, egui: &egui::Context) {
        use egui::widgets::DragValue;

        egui::Window::new("World")
            .default_open(false)
            .show(egui, |ui| {
                if let Some(intersection) = self.intersection {
                    ui.label("Intersection");
                    ui.label(format!("{}", intersection));
                }

                ui.heading("Camera");
                ui.horizontal(|ui| {
                    ui.label("Debug camera");
                    ui.toggle_value(&mut self.view_debug_camera, "View");
                    ui.toggle_value(&mut self.control_debug_camera, "Control");
                });

                if self.control_debug_camera {
                    let c = &mut self.debug_camera_controller;

                    egui::Grid::new("camera").show(ui, |ui| {
                        ui.label("position");

                        let mut pos = c.position;
                        let mut changed = false;
                        changed |= DragValue::new(&mut pos.x).ui(ui).changed();
                        changed |= DragValue::new(&mut pos.y).ui(ui).changed();
                        changed |= DragValue::new(&mut pos.z).ui(ui).changed();
                        if changed {
                            c.move_to(pos);
                        }
                        ui.end_row();

                        ui.label("pitch/yaw");
                        if DragValue::new(&mut c.pitch).speed(0.1).ui(ui).changed() {
                            c.smudge();
                        }
                        if DragValue::new(&mut c.yaw).speed(0.1).ui(ui).changed() {
                            c.smudge();
                        }
                    });
                };

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

                // ui.heading("Height");
                // ui.add(DragValue::new(&mut self.terrain_height_sample.x));
                // ui.add(DragValue::new(&mut self.terrain_height_sample.y));

                if let Some(fog) = &mut self.fog {
                    ui.heading("Fog");
                    ui.horizontal(|ui| {
                        ui.label("Density");
                        ui.add(DragValue::new(&mut self.fog_density).speed(0.01));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Range");
                        ui.add(DragValue::new(&mut fog.start).speed(10));
                        ui.add(DragValue::new(&mut fog.end).speed(10));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Color");
                        ui.add(DragValue::new(&mut fog.color.x).speed(0.01));
                        ui.add(DragValue::new(&mut fog.color.y).speed(0.01));
                        ui.add(DragValue::new(&mut fog.color.z).speed(0.01));
                    });

                    self.fog_density = self.fog_density.clamp(0.0, 1.0);
                }
            });

        self.compositor.debug_panel(egui);
    }
}
