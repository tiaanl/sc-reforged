use std::cell::RefCell;

use crate::{
    engine::{
        assets::{AssetError, Assets},
        gizmos::GizmosRenderer,
        renderer::Renderer,
        scene::Scene,
    },
    game::config::CampaignDef,
};
use camera::*;
use glam::{vec3, Quat, Vec2, Vec3};
use terrain::*;
use tracing::info;
use wgpu::util::DeviceExt;
use winit::{event::MouseButton, keyboard::KeyCode};

mod camera;
mod terrain;

#[derive(Default)]
pub struct WorldCamera {
    camera: RefCell<Camera>,

    pub pitch: f32,
    pub yaw: f32,

    pub velocity: Vec3,
}

impl WorldCamera {
    pub fn new(camera: Camera, pitch: f32, yaw: f32, velocity: Vec3) -> Self {
        Self {
            camera: RefCell::new(camera),
            pitch,
            yaw,
            velocity,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.camera.borrow_mut().aspect_ratio = (width as f32) / (height.max(1) as f32);
    }

    pub fn update(&mut self, delta_time: f32) {
        let mut camera = self.camera.borrow_mut();

        let v = camera.forward_vector();
        camera.position += v * self.velocity.z * delta_time;

        let v = camera.right_vector();
        camera.position += v * self.velocity.x * delta_time;

        let v = camera.up_vector();
        camera.position += v * self.velocity.y * delta_time;
    }

    fn calculate_rotation(&self) -> Quat {
        Quat::from_rotation_y(self.yaw.to_radians())
            * Quat::from_rotation_x(self.pitch.to_radians())
    }

    pub fn calculate_matrices(&self) -> Matrices {
        let mut camera = self.camera.borrow_mut();
        camera.rotation = self.calculate_rotation();
        camera.calculate_matrices()
    }
}

/// The [Scene] that renders the ingame world view.
pub struct WorldScene {
    _campaign_def: CampaignDef,

    world_camera: WorldCamera,
    camera_bind_group_layout: wgpu::BindGroupLayout,

    terrain: Terrain,

    last_mouse_position: Vec2,

    rotating_camera: Option<Vec2>,

    gizmos_renderer: GizmosRenderer,
}

impl WorldScene {
    pub fn new(
        assets: &Assets,
        renderer: &Renderer,
        campaign_def: CampaignDef,
    ) -> Result<Self, AssetError> {
        tracing::info!("Loading campaign \"{}\"...", campaign_def.title);

        let camera_bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("camera_bind_group_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let terrain = Terrain::new(assets, renderer, &camera_bind_group_layout)?;

        let camera = Camera::new(
            vec3(0.0, 1000.0, 1000.0),
            Quat::IDENTITY,
            45_f32.to_radians(),
            1.0,
            0.1,
            100_000.0,
        );
        let world_camera = WorldCamera::new(camera, 0.0, 0.0, Vec3::ZERO);

        let gizmos_renderer = GizmosRenderer::new(renderer);

        Ok(Self {
            _campaign_def: campaign_def,

            world_camera,
            camera_bind_group_layout,

            terrain,

            last_mouse_position: Vec2::ZERO,

            rotating_camera: None,

            gizmos_renderer,
        })
    }

    fn create_camera_bind_group(&self, renderer: &Renderer) -> (wgpu::Buffer, wgpu::BindGroup) {
        let matrices = self.world_camera.calculate_matrices();

        let buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("camera_uniform_buffer"),
                contents: bytemuck::cast_slice(&[matrices]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("camera_bind_group"),
                layout: &self.camera_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });

        (buffer, bind_group)
    }
}

impl Scene for WorldScene {
    fn resize(&mut self, width: u32, height: u32) {
        self.world_camera.resize(width, height);
    }

    fn on_key_pressed(&mut self, key: winit::keyboard::KeyCode) {
        const SPEED: f32 = 100.0;

        let mut velocity = self.world_camera.velocity;
        match key {
            KeyCode::KeyW => velocity.z += SPEED,
            KeyCode::KeyS => velocity.z -= SPEED,
            KeyCode::KeyD => velocity.x += SPEED,
            KeyCode::KeyA => velocity.x -= SPEED,
            KeyCode::KeyQ => velocity.y -= SPEED,
            KeyCode::KeyE => velocity.y += SPEED,
            _ => info!("key pressed: {key:?}"),
        }
        self.world_camera.velocity = velocity;
    }

    fn on_key_released(&mut self, key: winit::keyboard::KeyCode) {
        const SPEED: f32 = 100.0;

        let mut velocity = self.world_camera.velocity;
        match key {
            KeyCode::KeyW => velocity.z -= SPEED,
            KeyCode::KeyS => velocity.z += SPEED,
            KeyCode::KeyD => velocity.x -= SPEED,
            KeyCode::KeyA => velocity.x += SPEED,
            KeyCode::KeyQ => velocity.y += SPEED,
            KeyCode::KeyE => velocity.y -= SPEED,
            _ => info!("key released: {key:?}"),
        }
        self.world_camera.velocity = velocity;
    }

    fn on_mouse_moved(&mut self, position: glam::Vec2) {
        self.last_mouse_position = position;

        if let Some(pos) = self.rotating_camera {
            const SENSITIVITY: f32 = 0.5;
            let delta = (pos - self.last_mouse_position) * SENSITIVITY;

            self.world_camera.pitch -= delta.y;
            self.world_camera.yaw -= delta.x;

            self.rotating_camera = Some(self.last_mouse_position);
        }
    }

    fn on_mouse_pressed(&mut self, button: MouseButton) {
        if button == MouseButton::Left {
            self.rotating_camera = Some(self.last_mouse_position);
        }
    }
    fn on_mouse_released(&mut self, button: MouseButton) {
        if button == MouseButton::Left {
            self.rotating_camera = None;
        }
    }

    fn update(&mut self, delta_time: f32) {
        self.world_camera.update(delta_time);
        self.terrain.update(delta_time);
    }

    fn debug_panel(&mut self, egui: &egui::Context) {
        egui::Window::new("World").show(egui, |ui| {
            egui::Grid::new("world_info").show(ui, |ui| {
                ui.label("pitch");
                ui.add(egui::Label::new(format!("{}", self.world_camera.pitch)));
                ui.end_row();

                ui.label("yaw");
                ui.add(egui::Label::new(format!("{}", self.world_camera.yaw)));
                ui.end_row();
            });

            // ui.heading("Camera");
            // self.camera.debug_panel(ui);
            ui.heading("Terrain");
            self.terrain.debug_panel(ui);
        });
    }

    fn render(
        &self,
        renderer: &crate::engine::renderer::Renderer,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
    ) {
        let (_, camera_bind_group) = self.create_camera_bind_group(renderer);

        self.terrain
            .render(renderer, encoder, output, &camera_bind_group);

        self.gizmos_renderer
            .render(renderer, encoder, output, &camera_bind_group);
    }
}
