use std::path::PathBuf;

use crate::{
    engine::{
        gizmos::{GizmoVertex, GizmosRenderer},
        prelude::*,
        shaders::Shaders,
    },
    game::{
        asset_loader::{AssetError, AssetLoader},
        camera,
        config::{self, CampaignDef},
    },
};
use egui::Widget;
use glam::{Quat, Vec3, Vec4};
use terrain::*;

mod bounding_boxes;
mod entities;
mod terrain;

/// The [Scene] that renders the ingame world view.
pub struct WorldScene {
    asset_manager: AssetManager,
    _campaign_def: CampaignDef,

    use_debug_camera: bool,

    camera_controller: camera::FreeCameraController,
    camera: camera::Camera,
    debug_camera_controller: camera::FreeCameraController,
    debug_camera: camera::Camera,

    camera_matrices: Tracked<camera::Matrices>,
    gpu_camera: camera::GpuCamera,

    window_size: Vec2,
    intersection: Option<Vec3>,

    terrain: Terrain,

    entities: entities::Entities,

    gizmos_renderer: GizmosRenderer,
    gizmos_vertices: Vec<GizmoVertex>,

    // Input handling.
    under_mouse: UnderMouse,
    selected_entity: Option<usize>,

    new_id: String,
}

#[derive(Debug)]
enum UnderMouse {
    Nothing { position: Vec2 },
    Entity { entity_index: usize, position: Vec2 },
}

impl WorldScene {
    pub fn new(
        assets: &AssetLoader,
        asset_manager: AssetManager,
        renderer: &Renderer,
        campaign_def: CampaignDef,
    ) -> Result<Self, AssetError> {
        tracing::info!("Loading campaign \"{}\"...", campaign_def.title);

        // Load the campaign specification.
        let campaign = {
            let data = assets.load_string(
                PathBuf::from("campaign")
                    .join(&campaign_def.base_name)
                    .join(&campaign_def.base_name)
                    .with_extension("txt"),
            )?;
            let mut config = config::ConfigFile::new(&data);
            config::read_campaign(&mut config)?
        };

        let mut shaders = Shaders::default();
        camera::register_camera_shader(&mut shaders);

        let camera_from = campaign.view_initial.from.extend(2500.0);
        let camera_to = campaign.view_initial.to.extend(0.0);

        let mut camera_controller = camera::FreeCameraController::new(50.0, 0.4);
        camera_controller.move_to(camera_from);
        camera_controller.look_at(camera_to);
        let camera = camera::Camera::new(
            camera_from,
            Quat::IDENTITY,
            45.0_f32.to_radians(),
            1.0,
            1.0,
            100_000.0,
        );

        let debug_camera_controller = camera::FreeCameraController::new(50.0, 0.4);
        let debug_camera = camera::Camera::new(
            Vec3::ZERO,
            Quat::IDENTITY,
            45.0_f32.to_radians(),
            1.0,
            1.0,
            100_000.0,
        );

        let camera_matrices = camera.calculate_matrices().into();
        let gpu_camera = camera::GpuCamera::new(renderer);

        let terrain = Terrain::new(
            assets,
            renderer,
            &mut shaders,
            &campaign_def,
            &gpu_camera.bind_group_layout,
        )?;
        let mut entities = entities::Entities::new(
            asset_manager.clone(),
            renderer,
            &mut shaders,
            &gpu_camera.bind_group_layout,
        );

        {
            // Load the image defs.
            let data = assets.load_raw(r"config\image_defs.txt")?;
            let str = String::from_utf8(data).unwrap();
            let _image_defs = config::read_image_defs(&str);
        }

        if true {
            let path = PathBuf::from("maps")
                .join(format!("{}_final", campaign_def.base_name))
                .with_extension("mtf");
            let data = assets.load_string(&path)?;
            let mtf = config::read_mtf(&data);

            let mut to_spawn = mtf
                .objects
                .iter()
                .flat_map(|object| {
                    let path = PathBuf::from("models")
                        .join(&object.name)
                        .join(&object.name)
                        .with_extension("smf");

                    let model_handle = assets.load_smf_model(&path, renderer)?;

                    // let object_type = match object.typ.as_str() {
                    //     "4x4" => entities::ObjectType::_4x4,
                    //     "Scenery_Bush" => entities::ObjectType::SceneryBush,
                    //     "Scenery_Lit" => entities::ObjectType::SceneryLit,
                    //     "Scenery_Strip_Light" => entities::ObjectType::SceneryStripLight,
                    //     "Scenery" => entities::ObjectType::Scenery,
                    //     "Scenery_Alarm" => entities::ObjectType::Scenery,
                    //     "Structure_Building" => entities::ObjectType::Scenery,
                    //     "Structure_Fence" => entities::ObjectType::StructureFence,
                    //     "Structure_Swing_Door" => entities::ObjectType::StructureSwingDoor,
                    //     "Structure" => entities::ObjectType::Structure,
                    //     _ => tracing::warn!("Invalid object type: {}", object.typ),
                    // };

                    let entity =
                        entities::Entity::new(object.position, object.rotation, model_handle);

                    Ok::<_, AssetError>(entity)
                })
                .collect::<Vec<_>>();

            for entity in to_spawn.drain(..) {
                entities.spawn(entity);
            }
        }

        let gizmos_renderer = GizmosRenderer::new(renderer, &gpu_camera.bind_group_layout);

        Ok(Self {
            asset_manager,

            _campaign_def: campaign_def,

            use_debug_camera: false,

            camera_controller,
            camera,

            debug_camera_controller,
            debug_camera,

            camera_matrices,
            gpu_camera,

            window_size: Vec2::ZERO,
            intersection: None,

            terrain,
            entities,

            gizmos_renderer,
            gizmos_vertices: vec![],

            // Input handling.
            under_mouse: UnderMouse::Nothing {
                position: Vec2::ZERO,
            },
            selected_entity: None,

            new_id: String::new(),
        })
    }
}

impl Scene for WorldScene {
    fn resize(&mut self, width: u32, height: u32) {
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
                if let Some(entity_index) = self.entities.ray_intersection(&ray) {
                    UnderMouse::Entity {
                        entity_index,
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
                    UnderMouse::Entity {
                        entity_index,
                        position: p,
                    } if (position - p).length_squared() < 100.0 => {
                        // Set the selected entity.
                        self.selected_entity = Some(entity_index);
                    }
                    UnderMouse::Nothing { position: p }
                        if (position - p).length_squared() < 100.0 =>
                    {
                        self.selected_entity = None;
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

        if self.use_debug_camera {
            self.debug_camera_controller.update(input, delta_time);
            self.debug_camera_controller
                .update_camera_if_dirty(&mut self.debug_camera);
        } else {
            self.camera_controller.update(input, delta_time);
            if self
                .camera_controller
                .update_camera_if_dirty(&mut self.camera)
            {
                *self.camera_matrices = self.camera.calculate_matrices();
            }
        }

        // Highlight whatever we're hovering on.
        match self.under_mouse {
            UnderMouse::Nothing { .. } => self.entities.set_selected(None),
            UnderMouse::Entity { entity_index, .. } => {
                self.entities.set_selected(Some(entity_index))
            }
        }

        self.terrain.update(delta_time);

        {
            self.gizmos_vertices.append(&mut vec![
                // X+
                GizmoVertex::new(CENTER, RED),
                GizmoVertex::new(Vec3::X * GIZMO_SCALE, RED),
                // Y+
                GizmoVertex::new(CENTER, GREEN),
                GizmoVertex::new(Vec3::Y * GIZMO_SCALE, GREEN),
                // Z+
                GizmoVertex::new(CENTER, BLUE),
                GizmoVertex::new(Vec3::Z * GIZMO_SCALE, BLUE),
            ]);
        }
    }

    fn begin_frame(&mut self, _device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.use_debug_camera {
            self.gpu_camera
                .upload_matrices(queue, &self.debug_camera.calculate_matrices());
        } else {
            self.gpu_camera
                .upload_matrices(queue, &self.camera.calculate_matrices());
        }
    }

    fn render_frame(&self, frame: &mut Frame) {
        frame.clear_color_and_depth(
            wgpu::Color {
                r: 0.1,
                g: 0.2,
                b: 0.3,
                a: 1.0,
            },
            1.0,
        );

        self.terrain
            .render_frame(frame, &self.gpu_camera.bind_group);

        let more_vertices = self.terrain.render_normals();
        self.gizmos_renderer
            .render_frame(frame, &self.gpu_camera.bind_group, &more_vertices);

        self.entities
            .render_frame(frame, &self.gpu_camera.bind_group, &self.gizmos_renderer);

        self.gizmos_renderer.render_frame(
            frame,
            &self.gpu_camera.bind_group,
            &self.gizmos_vertices,
        );
    }

    fn end_frame(&mut self) {
        self.gizmos_vertices.clear();
    }

    fn debug_panel(&mut self, egui: &egui::Context) {
        use egui::widgets::DragValue;

        egui::Window::new("World").show(egui, |ui| {
            if let Some(intersection) = self.intersection {
                ui.label("Intersection");
                ui.label(format!("{}", intersection));
            }

            ui.heading("Camera");
            ui.toggle_value(&mut self.use_debug_camera, "Use debug camera");

            let c = if self.use_debug_camera {
                &mut self.debug_camera_controller
            } else {
                &mut self.camera_controller
            };

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

            // egui::Grid::new("world_info").show(ui, |ui| {
            //     ui.label("position");
            //     ui.add(egui::Label::new(format!(
            //         "{}, {}, {}",
            //         self.world_camera.camera.position.x,
            //         self.world_camera.camera.position.y,
            //         self.world_camera.camera.position.z,
            //     )));
            //     ui.end_row();

            //     ui.label("pitch");
            //     ui.add(egui::Label::new(format!("{}", self.world_camera.pitch)));
            //     ui.end_row();

            //     ui.label("yaw");
            //     ui.add(egui::Label::new(format!("{}", self.world_camera.yaw)));
            //     ui.end_row();
            // });

            // ui.heading("Camera");
            // self.camera.debug_panel(ui);

            ui.heading("Terrain");
            self.terrain.debug_panel(ui);

            ui.heading("Entities");
            self.entities.debug_panel(ui);

            ui.heading("Entity");
            if let Some(e) = self.selected_entity {
                ui.label(format!("{}", e));
                if let Some(entity) = self.entities.get_mut(e) {
                    ui.horizontal(|ui| {
                        ui.label("translation");
                        DragValue::new(&mut entity.translation.x).ui(ui);
                        DragValue::new(&mut entity.translation.y).ui(ui);
                        DragValue::new(&mut entity.translation.z).ui(ui);
                    });
                    ui.horizontal(|ui| {
                        ui.label("rotation");
                        DragValue::new(&mut entity.rotation.x).speed(0.01).ui(ui);
                        DragValue::new(&mut entity.rotation.y).speed(0.01).ui(ui);
                        DragValue::new(&mut entity.rotation.z).speed(0.01).ui(ui);
                    });

                    ui.label("Animation Set");
                    ui.text_edit_singleline(&mut self.new_id);
                    if ui.button("Add").clicked() {
                        let id: usize = self.new_id.parse().unwrap();
                        entity.animation_set.set.insert(id, Transform::default());
                        self.new_id.clear();
                    }
                    for (id, translation) in entity.animation_set.set.iter_mut() {
                        ui.label(format!("{}", id));
                        ui.horizontal(|ui| {
                            DragValue::new(&mut translation.translation.x)
                                .speed(0.01)
                                .ui(ui);
                            DragValue::new(&mut translation.translation.y)
                                .speed(0.01)
                                .ui(ui);
                            DragValue::new(&mut translation.translation.z)
                                .speed(0.01)
                                .ui(ui);
                        });
                        ui.horizontal(|ui| {
                            DragValue::new(&mut translation.rotation.x)
                                .speed(0.01)
                                .ui(ui);
                            DragValue::new(&mut translation.rotation.y)
                                .speed(0.01)
                                .ui(ui);
                            DragValue::new(&mut translation.rotation.z)
                                .speed(0.01)
                                .ui(ui);
                            DragValue::new(&mut translation.rotation.w)
                                .speed(0.01)
                                .ui(ui);
                        });
                    }

                    /*
                    if let Some(model) = self.asset_manager.get(entity.model) {
                        model.nodes.iter().enumerate().for_each(|(i, node)| {
                            ui.horizontal(|ui| {
                                ui.label(format!("{}", i));
                                ui.label(format!(
                                    "{} {} {}",
                                    node.transform.translation.x,
                                    node.transform.translation.y,
                                    node.transform.translation.z
                                ));
                                ui.label(format!(
                                    "{:.2} {:.2} {:.2} {:.2}",
                                    node.transform.rotation.x,
                                    node.transform.rotation.y,
                                    node.transform.rotation.z,
                                    node.transform.rotation.w,
                                ));
                            });
                        });
                    }
                    */
                }
            } else {
                ui.label("Nothing");
            }
        });
    }
}
