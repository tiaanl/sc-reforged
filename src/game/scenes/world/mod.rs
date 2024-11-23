use std::path::PathBuf;

use crate::{
    engine::{
        assets::{AssetError, AssetLoader},
        gizmos::{GizmoVertex, GizmosRenderer},
        renderer::Renderer,
        scene::Scene,
        shaders::Shaders,
    },
    game::{camera, config::CampaignDef},
};
use bounding_boxes::BoundingBoxes;
use glam::{vec3, Quat, Vec3, Vec4};
use terrain::*;
use tracing::{error, info};

mod bounding_boxes;
mod object;
mod terrain;
mod textures;

/// The [Scene] that renders the ingame world view.
pub struct WorldScene {
    _campaign_def: CampaignDef,

    camera_controller: camera::FreeCameraController,
    camera: camera::Camera,
    gpu_camera: camera::GpuCamera,

    terrain: Terrain,

    objects: object::Objects,

    gizmos_renderer: GizmosRenderer,
    gizmos_vertices: Vec<GizmoVertex>,

    bounding_boxes: bounding_boxes::BoundingBoxes,
}

impl WorldScene {
    pub fn new(
        assets: &AssetLoader,
        renderer: &Renderer,
        campaign_def: CampaignDef,
    ) -> Result<Self, AssetError> {
        tracing::info!("Loading campaign \"{}\"...", campaign_def.title);

        let mut shaders = Shaders::default();
        camera::register_camera_shader(&mut shaders);

        let terrain = Terrain::new(assets, renderer, &mut shaders, &campaign_def)?;
        let mut objects = object::Objects::new(renderer, &mut shaders);

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

                    info!("Loading mode: {}", &path.display());

                    let smf = match assets.load_smf(&path) {
                        Ok(smf) => smf,
                        Err(err) => {
                            error!("Could not load model {}: {}", path.display(), err);
                            return Err(err);
                        }
                    };

                    let model_handle = objects.model_renderer.add(renderer, assets, &smf);

                    // Convert the XYZ angles of the rotation to a quaternion.
                    let rotation = Quat::from_euler(
                        glam::EulerRot::XYZ,
                        object.rotation.x,
                        object.rotation.y,
                        object.rotation.z,
                    );

                    Ok(object::Object {
                        position: object.position,
                        rotation,
                        model_handle,
                    })
                })
                .collect::<Vec<_>>();

            for object in to_spawn.drain(..) {
                objects.spawn(object);
            }
        }

        let camera_controller = camera::FreeCameraController {
            movement_speed: 50.0,
            mouse_sensitivity: 0.4,
            ..Default::default()
        };

        let camera = camera::Camera::new(
            Vec3::ZERO,
            Quat::IDENTITY,
            45.0_f32.to_radians(),
            1.0,
            1.0,
            100_000.0,
        );
        let gpu_camera = camera::GpuCamera::new(renderer);

        let gizmos_renderer = GizmosRenderer::new(renderer);

        let mut bounding_boxes = BoundingBoxes::new(renderer).unwrap();
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
            gpu_camera,

            terrain,
            objects,

            gizmos_renderer,
            gizmos_vertices: vec![],

            bounding_boxes,
        })
    }
}

impl Scene for WorldScene {
    fn resize(&mut self, width: u32, height: u32) {
        self.camera.aspect_ratio = width as f32 / height.max(1) as f32;
    }

    fn on_input(&mut self, input: &crate::engine::input::InputState, delta_time: f32) {
        use camera::CameraController;
        self.camera_controller.on_input(input, delta_time);
    }

    fn update(&mut self, delta_time: f32) {
        // self.world_camera.update(delta_time);
        self.terrain.update(delta_time);

        const GIZMO_SCALE: f32 = 1000.0;
        const CENTER: Vec3 = Vec3::ZERO;
        const RED: Vec4 = Vec4::new(1.0, 0.0, 0.0, 1.0);
        const GREEN: Vec4 = Vec4::new(0.0, 1.0, 0.0, 1.0);
        const BLUE: Vec4 = Vec4::new(0.0, 0.0, 1.0, 1.0);
        self.gizmos_vertices = vec![
            // X+
            GizmoVertex::new(CENTER, RED),
            GizmoVertex::new(Vec3::X * GIZMO_SCALE, RED),
            // Y+
            GizmoVertex::new(CENTER, GREEN),
            GizmoVertex::new(Vec3::Y * GIZMO_SCALE, GREEN),
            // Z+
            GizmoVertex::new(CENTER, BLUE),
            GizmoVertex::new(Vec3::Z * GIZMO_SCALE, BLUE),
        ];
    }

    fn debug_panel(&mut self, egui: &egui::Context) {
        egui::Window::new("World").show(egui, |ui| {
            egui::Grid::new("world_info").show(ui, |ui| {
                // ui.label("position");
                // ui.add(egui::Label::new(format!(
                //     "{}, {}, {}",
                //     self.world_camera.camera.position.x,
                //     self.world_camera.camera.position.y,
                //     self.world_camera.camera.position.z,
                // )));
                // ui.end_row();

                // ui.label("pitch");
                // ui.add(egui::Label::new(format!("{}", self.world_camera.pitch)));
                // ui.end_row();

                // ui.label("yaw");
                // ui.add(egui::Label::new(format!("{}", self.world_camera.yaw)));
                // ui.end_row();
            });

            // ui.heading("Camera");
            // self.camera.debug_panel(ui);
            ui.heading("Terrain");
            self.terrain.debug_panel(ui);
        });
    }

    fn render(
        &mut self,
        renderer: &crate::engine::renderer::Renderer,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        {
            // Update the camera.
            use crate::game::camera::CameraController;
            self.camera_controller.update_camera(&mut self.camera);
            let matrices = self.camera.calculate_matrices();
            self.gpu_camera.upload_matrices(renderer, matrices);
        }

        self.terrain
            .render(renderer, encoder, view, &self.gpu_camera.bind_group);

        let mut more_vertices = self.terrain.render_normals();
        self.gizmos_vertices.append(&mut more_vertices);
        let mut more_vertices = self.terrain.render_normals_lookup();
        self.gizmos_vertices.append(&mut more_vertices);

        self.objects.render(
            renderer,
            encoder,
            view,
            &self.gpu_camera.bind_group,
            // &mut self.bounding_boxes,
        );

        self.bounding_boxes
            .render_all(renderer, encoder, view, &self.gpu_camera.bind_group);
        self.bounding_boxes.clear();

        self.gizmos_renderer.render(
            renderer,
            encoder,
            view,
            &self.gpu_camera.bind_group,
            &self.gizmos_vertices,
        );
        self.gizmos_vertices.clear();
    }
}
