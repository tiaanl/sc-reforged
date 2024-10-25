use crate::{
    engine::{
        assets::{AssetError, Assets},
        renderer::Renderer,
        scene::Scene,
    },
    game::config::CampaignDef,
};
use camera::*;
use glam::{vec3, Quat, Vec2};
use terrain::*;
use tracing::info;
use wgpu::util::DeviceExt;
use winit::{event::MouseButton, keyboard::KeyCode};

mod camera;
mod terrain;

pub struct WorldCamera {
    pub pitch: f32,
    pub yaw: f32,
}

impl WorldCamera {
    pub fn new(pitch: f32, yaw: f32) -> Self {
        WorldCamera { pitch, yaw }
    }

    pub fn rotate_right(&mut self, yaw: f32) {
        let mut yaw = self.yaw + yaw;
        while yaw > 360.0 {
            yaw -= 360.0
        }
        while yaw < 0.0 {
            yaw += 360.0
        }
        self.yaw = yaw;
    }

    pub fn rotate_up(&mut self, pitch: f32) {
        let mut pitch = self.pitch + pitch;
        while pitch > 360.0 {
            pitch -= 360.0
        }
        while pitch < 0.0 {
            pitch += 360.0
        }
        self.pitch = pitch;
    }

    pub fn calculate_rotation(&self) -> Quat {
        Quat::from_rotation_y(self.yaw.to_radians())
            * Quat::from_rotation_x(self.pitch.to_radians())
    }
}

/// The [Scene] that renders the ingame world view.
pub struct WorldScene {
    _campaign_def: CampaignDef,

    camera: Camera,
    world_camera: WorldCamera,
    camera_bind_group_layout: wgpu::BindGroupLayout,

    terrain: Terrain,

    last_mouse_position: Vec2,

    rotating_camera: Option<Vec2>,

    moving_forward: f32,
    moving_right: f32,
    moving_up: f32,
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
        let world_camera = WorldCamera::new(0.0, 0.0);

        Ok(Self {
            _campaign_def: campaign_def,

            camera,
            world_camera,
            camera_bind_group_layout,

            terrain,

            last_mouse_position: Vec2::ZERO,

            rotating_camera: None,

            moving_forward: 0.0,
            moving_right: 0.0,
            moving_up: 0.0,
        })
    }

    fn create_camera_bind_group(&self, renderer: &Renderer) -> (wgpu::Buffer, wgpu::BindGroup) {
        let matrices = self.camera.calculate_matrices();

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
        self.camera.aspect_ratio = (width as f32) / (height.max(1) as f32);
    }

    fn on_key_pressed(&mut self, key: winit::keyboard::KeyCode) {
        const SPEED: f32 = 100.0;
        match key {
            KeyCode::KeyW => self.moving_forward += SPEED,
            KeyCode::KeyS => self.moving_forward -= SPEED,
            KeyCode::KeyA => self.moving_right -= SPEED,
            KeyCode::KeyD => self.moving_right += SPEED,
            KeyCode::KeyQ => self.moving_up -= SPEED,
            KeyCode::KeyE => self.moving_up += SPEED,
            _ => info!("key pressed: {key:?}"),
        }
    }

    fn on_key_released(&mut self, key: winit::keyboard::KeyCode) {
        const SPEED: f32 = 100.0;
        match key {
            KeyCode::KeyW => self.moving_forward -= SPEED,
            KeyCode::KeyS => self.moving_forward += SPEED,
            KeyCode::KeyA => self.moving_right += SPEED,
            KeyCode::KeyD => self.moving_right -= SPEED,
            KeyCode::KeyQ => self.moving_up += SPEED,
            KeyCode::KeyE => self.moving_up -= SPEED,
            _ => info!("key released: {key:?}"),
        }
    }

    fn on_mouse_moved(&mut self, position: glam::Vec2) {
        self.last_mouse_position = position;

        if let Some(pos) = self.rotating_camera {
            const SENSITIVITY: f32 = 0.7777;
            let delta = (pos - self.last_mouse_position) * SENSITIVITY;

            self.world_camera.pitch += delta.y;
            self.world_camera.yaw += delta.x;
            self.camera.rotation = self.world_camera.calculate_rotation();

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
        self.terrain.update(delta_time);

        self.camera.position += self.camera.forward_vector() * self.moving_forward * delta_time;
        self.camera.position += self.camera.right_vector() * self.moving_right * delta_time;
        self.camera.position += self.camera.up_vector() * self.moving_up * delta_time;
    }

    fn debug_panel(&mut self, egui: &egui::Context) {
        egui::Window::new("World").show(egui, |ui| {
            egui::Grid::new("world_info").show(ui, |ui| {
                ui.label("pitch");
                ui.add(egui::Slider::new(&mut self.world_camera.pitch, 0.0..=360.0));
                ui.end_row();

                ui.label("yaw");
                ui.add(egui::Slider::new(&mut self.world_camera.yaw, 0.0..=360.0));
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
    }
}
