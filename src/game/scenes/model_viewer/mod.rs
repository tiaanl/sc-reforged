use core::f32;

use glam::{Quat, Vec3, Vec4};
use shadow_company_tools::smf;

use crate::{
    engine::{
        gizmos::{GizmoVertex, GizmosRenderer},
        prelude::*,
    },
    game::{
        asset_loader::{AssetError, AssetLoader},
        camera,
        mesh_renderer::MeshRenderer,
        model::Model,
    },
};

pub struct ModelViewer {
    asset_store: AssetStore,

    mesh_renderer: MeshRenderer,
    model: Handle<Model>,
    model_position: Vec3,
    model_rotation: Vec3,

    gizmos: GizmosRenderer,

    view_debug_camera: bool,
    control_debug_camera: bool,
    camera_controller: camera::ArcBallCameraController,
    debug_camera_controller: camera::FreeCameraController,
    camera: camera::Camera,
    gpu_camera: camera::GpuCamera,
}

impl Asset for smf::Model {}

impl ModelViewer {
    pub fn new(
        asset_loader: &AssetLoader,
        asset_store: AssetStore,
        renderer: &Renderer,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, AssetError> {
        let mut shaders = Shaders::default();
        camera::register_camera_shader(&mut shaders);

        let camera = camera::Camera {
            position: Vec3::new(100.0, -2500.0, 1000.0),
            rotation: Quat::IDENTITY,
            fov: 45.0,
            aspect_ratio: 1.0,
            near: 1.0,
            far: 10_000.0,
        };

        let gpu_camera = camera::GpuCamera::new(renderer);

        let mesh_renderer = MeshRenderer::new(
            asset_store.clone(),
            renderer,
            &mut shaders,
            &gpu_camera.bind_group_layout,
            &gpu_camera.bind_group_layout,
        );

        let model = asset_loader.load_smf(path, renderer)?;

        let gizmos = GizmosRenderer::new(renderer, &gpu_camera.bind_group_layout);

        const CAM_SPEED: f32 = 10.0;
        const MOUSE_SENSITIVITY: f32 = 0.4;
        let debug_camera_controller =
            camera::FreeCameraController::new(CAM_SPEED, MOUSE_SENSITIVITY);
        let mut camera_controller = camera::ArcBallCameraController::new(MOUSE_SENSITIVITY);
        camera_controller.distance = 1_500.0;

        Ok(Self {
            asset_store,

            mesh_renderer,
            model,
            model_position: Vec3::ZERO,
            model_rotation: Vec3::ZERO,

            gizmos,

            view_debug_camera: false,
            control_debug_camera: false,
            debug_camera_controller,
            camera_controller,
            camera,
            gpu_camera,
        })
    }
}

impl Scene for ModelViewer {
    fn resize(&mut self, width: u32, height: u32) {
        self.camera.aspect_ratio = width as f32 / height.max(1) as f32;
    }

    fn update(&mut self, delta_time: f32, input: &InputState) {
        if self.control_debug_camera {
            self.debug_camera_controller.update(input, delta_time);
        } else {
            self.camera_controller.on_input(input, delta_time);
        }
    }

    fn begin_frame(&mut self, _device: &wgpu::Device, queue: &wgpu::Queue) {
        let changed = if self.view_debug_camera {
            self.debug_camera_controller
                .update_camera_if_dirty(&mut self.camera)
        } else {
            self.camera_controller
                .update_camera_if_changed(&mut self.camera)
        };

        if changed {
            let matrices = self.camera.calculate_matrices();
            self.gpu_camera
                .upload_matrices(queue, &matrices, self.camera.position);
        }
    }

    fn render_frame(&self, frame: &mut Frame) {
        let Some(model) = self.asset_store.get(self.model) else {
            tracing::error!("Invalid model");
            return;
        };

        frame.clear_color_and_depth(
            wgpu::Color {
                r: 0.1,
                g: 0.2,
                b: 0.3,
                a: 1.0,
            },
            1.0,
        );

        let transform = Transform::from_translation(self.model_position)
            .with_euler_rotation(self.model_rotation);
        let list = MeshRenderer::mesh_list_from_model(&model, transform);

        self.mesh_renderer.render_multiple(
            frame,
            &self.gpu_camera.bind_group,
            &self.gpu_camera.bind_group,
            &list,
        );

        const AXIS_SIZE: f32 = 100.0;
        let mut vertices = vec![
            // X
            GizmoVertex::new(Vec3::ZERO, Vec4::new(1.0, 0.0, 0.0, 1.0)),
            GizmoVertex::new(Vec3::X * AXIS_SIZE, Vec4::new(1.0, 0.0, 0.0, 1.0)),
            // Y
            GizmoVertex::new(Vec3::ZERO, Vec4::new(0.0, 1.0, 0.0, 1.0)),
            GizmoVertex::new(Vec3::Y * AXIS_SIZE, Vec4::new(0.0, 1.0, 0.0, 1.0)),
            // Z
            GizmoVertex::new(Vec3::ZERO, Vec4::new(0.0, 0.0, 1.0, 1.0)),
            GizmoVertex::new(Vec3::Z * AXIS_SIZE, Vec4::new(0.0, 0.0, 1.0, 1.0)),
        ];

        // Render a line from the camera position to the center.
        {
            let (position, rotation) = self.camera_controller.position_and_rotation();
            vertices.push(GizmoVertex::new(position, Vec4::new(1.0, 1.0, 1.0, 1.0)));
            vertices.push(GizmoVertex::new(
                position + rotation * Vec3::Y * 100.0,
                Vec4::new(1.0, 1.0, 1.0, 1.0),
            ));
        }

        self.gizmos
            .render(frame, &self.gpu_camera.bind_group, &vertices);
    }

    fn debug_panel(&mut self, egui: &egui::Context) {
        egui::Window::new("Model Viwer").show(egui, |ui| {
            ui.heading("Camera");
            ui.checkbox(&mut self.view_debug_camera, "View debug camera");
            ui.checkbox(&mut self.control_debug_camera, "Control debug camera");
            egui::Grid::new("world_info").show(ui, |ui| {
                if self.control_debug_camera {
                    ui.label("position");
                    ui.add(
                        egui::DragValue::new(&mut self.debug_camera_controller.position.x)
                            .speed(0.1),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.debug_camera_controller.position.y)
                            .speed(0.1),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.debug_camera_controller.position.z)
                            .speed(0.1),
                    );
                    ui.end_row();

                    ui.label("yaw");
                    ui.add(egui::DragValue::new(&mut self.debug_camera_controller.yaw).speed(0.1));
                    ui.end_row();

                    ui.label("pitch");
                    ui.add(
                        egui::DragValue::new(&mut self.debug_camera_controller.pitch).speed(0.1),
                    );
                    ui.end_row();
                } else {
                    ui.label("yaw");
                    ui.add(egui::DragValue::new(&mut self.camera_controller.yaw).speed(0.1));
                    ui.end_row();

                    ui.label("pitch");
                    ui.add(egui::DragValue::new(&mut self.camera_controller.pitch).speed(0.1));
                    ui.end_row();

                    ui.label("distance");
                    let camera_speed = self.camera_controller.distance / 100.0;
                    ui.add(
                        egui::DragValue::new(&mut self.camera_controller.distance)
                            .speed(camera_speed),
                    );
                    ui.end_row();
                }
                ui.end_row();
            });

            ui.heading("Model");
            egui::Grid::new("model_info").show(ui, |ui| {
                ui.label("Position");
                ui.add(egui::DragValue::new(&mut self.model_position.x).speed(0.1));
                ui.add(egui::DragValue::new(&mut self.model_position.y).speed(0.1));
                ui.add(egui::DragValue::new(&mut self.model_position.z).speed(0.1));
                ui.end_row();

                ui.label("Rotation");
                ui.add(egui::DragValue::new(&mut self.model_rotation.x).speed(0.01));
                ui.add(egui::DragValue::new(&mut self.model_rotation.y).speed(0.01));
                ui.add(egui::DragValue::new(&mut self.model_rotation.z).speed(0.01));
                ui.end_row();
            });
        });
    }
}
