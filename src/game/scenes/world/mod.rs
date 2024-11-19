use std::path::PathBuf;

use crate::{
    engine::{
        assets::{AssetError, Assets},
        gizmos::{GizmoVertex, GizmosRenderer},
        renderer::Renderer,
        scene::Scene,
        shaders::Shaders,
    },
    game::config::CampaignDef,
};
use ahash::HashSet;
use bounding_boxes::BoundingBoxes;
use glam::{vec3, Mat4, Quat, Vec2, Vec3, Vec4};
use terrain::*;
use tracing::error;
use winit::{event::MouseButton, keyboard::KeyCode};

mod bounding_boxes;
mod camera;
mod models;
mod object;
mod terrain;
mod textures;

#[inline]
fn sinister_transform() -> Mat4 {
    Mat4::from_cols(
        Vec4::new(-1.0, 0.0, 0.0, 0.0),
        Vec4::new(0.0, 0.0, -1.0, 0.0),
        Vec4::new(0.0, 1.0, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 0.0, 1.0),
    )
    // Mat4::IDENTITY
}

/*
#[derive(Default)]
pub struct WorldCamera {
    camera: Camera,

    pub pitch: f32,
    pub yaw: f32,

    pub velocity: Vec3,
}

impl WorldCamera {
    pub fn new(camera: Camera, pitch: f32, yaw: f32, velocity: Vec3) -> Self {
        Self {
            camera: camera,
            pitch,
            yaw,
            velocity,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.camera.aspect_ratio = (width as f32) / (height.max(1) as f32);
    }

    pub fn update(&mut self, delta_time: f32) {
        let ref mut camera = self.camera;

        let v = camera.forward_vector();
        camera.position += v * self.velocity.z * delta_time;

        let v = camera.right_vector();
        camera.position += v * self.velocity.x * delta_time;

        let v = camera.up_vector();
        camera.position += v * self.velocity.y * delta_time;
    }

    fn calculate_rotation(&mut self) -> Quat {
        let pitch = Quat::from_rotation_x(self.pitch.to_radians());
        let yaw = Quat::from_rotation_z(self.yaw.to_radians());
        yaw * pitch

        // let pitch_rotation = Quat::from_rotation_x(self.pitch.to_radians());
        // let yaw_rotation = Quat::from_rotation_y(self.yaw.to_radians());
        // yaw_rotation * pitch_rotation
    }

    pub fn calculate_matrices(&mut self) -> Matrices {
        self.camera.rotation = self.calculate_rotation();
        self.camera.calculate_matrices()
    }
}
*/

/// The [Scene] that renders the ingame world view.
pub struct WorldScene {
    _campaign_def: CampaignDef,

    // world_camera: WorldCamera,
    camera: camera::Camera,
    gpu_camera: camera::GpuCamera,
    camera_pitch: f32,
    camera_yaw: f32,

    terrain: Terrain,

    objects: object::Objects,

    held_keys: HashSet<KeyCode>,

    last_mouse_position: Vec2,

    rotating_camera: Option<Vec2>,

    gizmos_renderer: GizmosRenderer,
    gizmos_vertices: Vec<GizmoVertex>,

    bounding_boxes: bounding_boxes::BoundingBoxes,
}

impl WorldScene {
    pub fn new(
        assets: &Assets,
        renderer: &Renderer,
        campaign_def: CampaignDef,
    ) -> Result<Self, AssetError> {
        tracing::info!("Loading campaign \"{}\"...", campaign_def.title);

        let mut shaders = Shaders::default();
        shaders.add_module(include_str!("camera.wgsl"), "camera.wgsl");

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

            let models = &mut objects.models;

            let mut to_spawn = mtf
                .objects
                .iter()
                .flat_map(|object| {
                    let path = PathBuf::from("models")
                        .join(&object.name)
                        .join(&object.name)
                        .with_extension("smf");
                    let smf = match assets.load_smf(&path) {
                        Ok(smf) => smf,
                        Err(err) => {
                            error!("Could not load model {}: {}", path.display(), err);
                            return Err(err);
                        }
                    };

                    let model_handle = models.insert(renderer, assets, &smf);

                    // let model = match Self::smf_to_model(renderer, assets, textures, smf) {
                    //     Ok(model) => model,
                    //     Err(err) => {
                    //         error!("Could not load model: {}", path.display());
                    //         return Err(err);
                    //     }
                    // };
                    // let model_handle = models.insert(model);

                    Ok(object::Object {
                        position: object.position,
                        rotation: object.rotation,
                        model_handle,
                    })
                })
                .collect::<Vec<_>>();

            for object in to_spawn.drain(..) {
                objects.spawn(object);
            }
        }

        if true {
            let smf = assets.load_smf(r"models\alvhqd-hummer\alvhqd-hummer.smf")?;
            // let smf = assets.load_smf(r"models\anstbk-chem_tank\anstbk-chem_tank.smf")?;
            let model_handle = objects.models.insert(renderer, assets, &smf);
            objects.spawn(object::Object::new(Vec3::ZERO, Vec3::ZERO, model_handle));
        }

        // let camera = Camera::new(
        //     Vec3::Z * 1000.0,
        //     Quat::IDENTITY,
        //     45_f32.to_radians(),
        //     1.0,
        //     0.1,
        //     100_000.0,
        // );
        // let world_camera = WorldCamera::new(camera, 0.0, 0.0, Vec3::ZERO);
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

            camera,
            gpu_camera,
            camera_pitch: 0.0,
            camera_yaw: 0.0,

            terrain,
            objects,

            held_keys: ahash::HashSet::default(),

            last_mouse_position: Vec2::ZERO,

            rotating_camera: None,

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

    fn on_key_pressed(&mut self, key: KeyCode) {
        self.held_keys.insert(key);
    }

    fn on_key_released(&mut self, key: KeyCode) {
        self.held_keys.remove(&key);
    }

    fn on_mouse_moved(&mut self, position: glam::Vec2) {
        self.last_mouse_position = position;

        if let Some(pos) = self.rotating_camera {
            const SENSITIVITY: f32 = 0.1;
            let delta = (pos - self.last_mouse_position) * SENSITIVITY;
            self.camera_pitch += delta.y;
            self.camera_yaw -= delta.x;

            self.rotating_camera = Some(self.last_mouse_position);
        }
    }

    fn on_mouse_pressed(&mut self, button: MouseButton) {
        if button == MouseButton::Right {
            self.rotating_camera = Some(self.last_mouse_position);
        }
    }

    fn on_mouse_released(&mut self, button: MouseButton) {
        if button == MouseButton::Right {
            self.rotating_camera = None;
        }
    }

    fn update(&mut self, delta_time: f32) {
        const SPEED: f32 = 50.0;
        if self.held_keys.contains(&KeyCode::KeyW) {
            self.camera.move_forward(SPEED * delta_time);
        }
        if self.held_keys.contains(&KeyCode::KeyS) {
            self.camera.move_forward(-SPEED * delta_time);
        }
        if self.held_keys.contains(&KeyCode::KeyA) {
            self.camera.move_right(SPEED * delta_time);
        }
        if self.held_keys.contains(&KeyCode::KeyD) {
            self.camera.move_right(-SPEED * delta_time);
        }
        if self.held_keys.contains(&KeyCode::KeyE) {
            self.camera.move_up(SPEED * delta_time);
        }
        if self.held_keys.contains(&KeyCode::KeyQ) {
            self.camera.move_up(-SPEED * delta_time);
        }

        let rot = Quat::from_rotation_z(self.camera_yaw.to_radians())
            * Quat::from_rotation_x(self.camera_pitch.to_radians());
        self.camera.rotation = rot;

        // self.camera.rotation = Quat::from_euler(
        //     glam::EulerRot::XYZ,
        //     self.camera_pitch.to_radians(),
        //     0.0,
        //     self.camera_yaw.to_radians(),
        // );

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
        let matrices = if false {
            let aspect =
                renderer.surface_config.width as f32 / renderer.surface_config.height.max(1) as f32;
            let view = Mat4::from_cols(
                Vec4::from((Vec3::NEG_X, 0.0)),
                Vec4::from((Vec3::Z, 0.0)),
                Vec4::from((Vec3::NEG_Y, 0.0)),
                Vec4::from((Vec3::ZERO, 1.0)),
            );
            let view = Mat4::from_euler(
                glam::EulerRot::XYZ,
                self.camera_pitch.to_radians(),
                self.camera_yaw.to_radians(),
                0.0,
            ) * view;

            // self.view_matrix = view;

            camera::Matrices {
                projection: Mat4::perspective_lh(45.0_f32.to_radians(), aspect, 1.0, 100_000.0)
                    .to_cols_array_2d(),
                view: view.to_cols_array_2d(),
            }
        } else {
            self.camera.calculate_matrices()
        };

        self.gpu_camera.upload_matrices(renderer, matrices);

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
            &mut self.bounding_boxes,
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
