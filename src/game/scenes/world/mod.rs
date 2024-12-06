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
        config::CampaignDef,
    },
};
use bounding_boxes::BoundingBoxes;
use glam::{vec3, Quat, Vec3, Vec4};
use terrain::*;

mod bounding_boxes;
mod entities;
mod terrain;

/// The [Scene] that renders the ingame world view.
pub struct WorldScene {
    _campaign_def: CampaignDef,

    camera_controller: camera::FreeCameraController,
    camera: camera::Camera,
    camera_matrices: Tracked<camera::Matrices>,
    gpu_camera: camera::GpuCamera,

    window_size: Vec2,
    intersection: Option<Vec3>,

    terrain: Terrain,

    objects: entities::Entities,

    gizmos_renderer: GizmosRenderer,
    gizmos_vertices: Vec<GizmoVertex>,

    bounding_boxes: bounding_boxes::BoundingBoxes,
}

impl WorldScene {
    pub fn new(
        assets: &AssetLoader,
        asset_manager: AssetManager,
        renderer: &Renderer,
        campaign_def: CampaignDef,
    ) -> Result<Self, AssetError> {
        tracing::info!("Loading campaign \"{}\"...", campaign_def.title);

        let mut shaders = Shaders::default();
        camera::register_camera_shader(&mut shaders);

        let camera = camera::Camera::new(
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
            asset_manager,
            renderer,
            &mut shaders,
            &gpu_camera.bind_group_layout,
        );

        {
            // Load the image defs.
            let data = assets.load_raw(r"config\image_defs.txt")?;
            let str = String::from_utf8(data).unwrap();
            let _image_defs = crate::game::config::read_image_defs(&str);
        }

        if true {
            let path = PathBuf::from("maps")
                .join(format!("{}_final", campaign_def.base_name))
                .with_extension("mtf");
            let data = assets.load_config_file(&path)?;
            let mtf = crate::game::config::read_mtf(&data);

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

                    let entity = entities::Entity::new(
                        Transform::new(
                            object.position,
                            Quat::from_euler(
                                glam::EulerRot::XYZ,
                                object.rotation.x,
                                object.rotation.y,
                                object.rotation.z,
                            ),
                        ),
                        model_handle,
                    );

                    Ok::<_, AssetError>(entity)
                })
                .collect::<Vec<_>>();

            for entity in to_spawn.drain(..) {
                entities.spawn(entity);
            }
        }

        let camera_controller = camera::FreeCameraController::new(50.0, 0.4);

        let gizmos_renderer = GizmosRenderer::new(renderer, &gpu_camera.bind_group_layout);

        let mut bounding_boxes =
            BoundingBoxes::new(renderer, &gpu_camera.bind_group_layout).unwrap();
        bounding_boxes.insert(
            vec3(300.0, 300.0, 300.0),
            Quat::from_rotation_z(45.0_f32.to_radians()),
            vec3(0.0, 0.0, 0.0),
            vec3(100.0, 100.0, 100.0),
        );
        bounding_boxes.insert(
            Vec3::ZERO,
            Quat::IDENTITY,
            vec3(100.0, 100.0, 100.0),
            vec3(300.0, 300.0, 300.0),
        );

        Ok(Self {
            _campaign_def: campaign_def,

            camera_controller,
            camera,
            camera_matrices,
            gpu_camera,

            window_size: Vec2::ZERO,
            intersection: None,

            terrain,
            objects: entities,

            gizmos_renderer,
            gizmos_vertices: vec![],

            bounding_boxes,
        })
    }
}

impl Scene for WorldScene {
    fn resize(&mut self, width: u32, height: u32) {
        self.window_size = Vec2::new(width as f32, height as f32);
        self.camera.aspect_ratio = width as f32 / height.max(1) as f32;
    }

    fn update(&mut self, delta_time: f32, input: &InputState) {
        const GIZMO_SCALE: f32 = 1000.0;
        const CENTER: Vec3 = Vec3::ZERO;
        const RED: Vec4 = Vec4::new(1.0, 0.0, 0.0, 1.0);
        const GREEN: Vec4 = Vec4::new(0.0, 1.0, 0.0, 1.0);
        const BLUE: Vec4 = Vec4::new(0.0, 0.0, 1.0, 1.0);

        self.camera_controller.update(input, delta_time);
        if self
            .camera_controller
            .update_camera_if_dirty(&mut self.camera)
        {
            *self.camera_matrices = self.camera.calculate_matrices();
        }

        if let Some(Vec2 { x, y }) = input.mouse_position() {
            let ndc = Vec2::new(
                x / self.window_size.x.max(1.0) * 2.0 - 1.0,
                (1.0 - y / self.window_size.y.max(1.0)) * 2.0 - 1.0,
            );

            let ray = self.camera.generate_ray(ndc);

            let plane = camera::Plane {
                point: Vec3::ZERO,
                normal: Vec3::Z,
            };
            self.intersection = ray.intersect_plane(&plane);

            if let Some(intersection) = self.intersection {
                self.gizmos_vertices = vec![
                    // X+
                    GizmoVertex::new(intersection, RED),
                    GizmoVertex::new(intersection + Vec3::X * 100.0, RED),
                    // Y+
                    GizmoVertex::new(intersection, GREEN),
                    GizmoVertex::new(intersection + Vec3::Y * 100.0, GREEN),
                    // Z+
                    GizmoVertex::new(intersection, BLUE),
                    GizmoVertex::new(intersection + Vec3::Z * 100.0, BLUE),
                ];
            }
        }

        // self.world_camera.update(delta_time);
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

    fn render_update(&mut self, _device: &wgpu::Device, queue: &wgpu::Queue) {
        self.camera_matrices.if_changed(|m| {
            self.gpu_camera.upload_matrices(queue, m);
        });
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

        // let mut more_vertices = self.terrain.render_normals();
        // self.gizmos_vertices.append(&mut more_vertices);
        // let mut more_vertices = self.terrain.render_normals_lookup();
        // self.gizmos_vertices.append(&mut more_vertices);

        self.objects
            .render_frame(frame, &self.gpu_camera.bind_group);

        // self.bounding_boxes.clear();

        self.gizmos_renderer.render_frame(
            frame,
            &self.gpu_camera.bind_group,
            &self.gizmos_vertices,
        );

        // self.gizmos_vertices.clear();
    }

    fn debug_panel(&mut self, egui: &egui::Context) {
        egui::Window::new("World").show(egui, |ui| {
            if let Some(intersection) = self.intersection {
                ui.label("Intersection");
                ui.label(format!("{}", intersection));
            }

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
        });
    }
}
