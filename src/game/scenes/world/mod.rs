use crate::{
    engine::{
        assets::{AssetError, Assets},
        renderer::Renderer,
        scene::Scene,
    },
    game::config::CampaignDef,
};
use camera::*;
use glam::{vec3, Mat4, Quat, Vec2};
use terrain::*;
use tracing::info;
use wgpu::util::DeviceExt;
use winit::{event::MouseButton, keyboard::KeyCode};

mod camera;
mod terrain;

/// The [Scene] that renders the ingame world view.
pub struct WorldScene {
    campaign_def: CampaignDef,

    camera: Camera,
    camera_bind_group_layout: wgpu::BindGroupLayout,

    model: [[f32; 4]; 4],
    model_bind_group_layout: wgpu::BindGroupLayout,

    terrain: Terrain,

    last_mouse_position: Vec2,

    moving_camera: Option<Vec2>,

    moving_forward: f32,
    moving_right: f32,
    moving_up: f32,

    camera_yaw: f32,
    camera_pitch: f32,
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

        let model = Mat4::IDENTITY.to_cols_array_2d();
        let model_bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("model_bind_group_layout"),
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

        let camera = Camera::from_position_rotation(vec3(0.0, 1000.0, 1000.0), Quat::IDENTITY);

        let terrain = Terrain::new(
            assets,
            renderer,
            &camera_bind_group_layout,
            &model_bind_group_layout,
        )?;

        Ok(Self {
            campaign_def,

            camera,
            camera_bind_group_layout,

            model,
            model_bind_group_layout,

            terrain,

            last_mouse_position: Vec2::ZERO,

            moving_camera: None,

            moving_forward: 0.0,
            moving_right: 0.0,
            moving_up: 0.0,

            camera_yaw: 0.0,
            camera_pitch: 0.0,
        })
    }

    fn create_camera_bind_group(&self, renderer: &Renderer) -> (wgpu::Buffer, wgpu::BindGroup) {
        let matrices = self.camera.create_matrices();

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
        self.camera.resize(width, height);
    }

    fn on_key_pressed(&mut self, key: winit::keyboard::KeyCode) {
        const SPEED: f32 = 100.0;
        match key {
            KeyCode::KeyW => self.moving_forward += SPEED,
            KeyCode::KeyS => self.moving_forward += -SPEED,
            KeyCode::KeyA => self.moving_right += -SPEED,
            KeyCode::KeyD => self.moving_right += SPEED,
            KeyCode::KeyQ => self.moving_up += -SPEED,
            KeyCode::KeyE => self.moving_up += SPEED,
            _ => info!("key pressed: {key:?}"),
        }
    }

    fn on_key_released(&mut self, key: winit::keyboard::KeyCode) {
        const SPEED: f32 = 100.0;
        match key {
            KeyCode::KeyW => self.moving_forward -= SPEED,
            KeyCode::KeyS => self.moving_forward -= -SPEED,
            KeyCode::KeyA => self.moving_right -= -SPEED,
            KeyCode::KeyD => self.moving_right -= SPEED,
            KeyCode::KeyQ => self.moving_up -= -SPEED,
            KeyCode::KeyE => self.moving_up -= SPEED,
            _ => info!("key released: {key:?}"),
        }
    }

    fn on_mouse_moved(&mut self, position: glam::Vec2) {
        self.last_mouse_position = position;

        if let Some(pos) = self.moving_camera {
            let delta = pos - self.last_mouse_position;
            self.camera_yaw += delta.x;
            self.camera_pitch += delta.y;
            self.moving_camera = Some(self.last_mouse_position);
        }
    }

    fn on_mouse_pressed(&mut self, button: MouseButton) {
        if button == MouseButton::Left {
            self.moving_camera = Some(self.last_mouse_position);
        }
    }
    fn on_mouse_released(&mut self, button: MouseButton) {
        if button == MouseButton::Left {
            self.moving_camera = None;
        }
    }

    fn update(&mut self, delta_time: f32) {
        self.terrain.update(delta_time);

        // Build a rotation based on the yaw and pitch values.
        let rotation = {
            let yaw_rotation = Quat::from_rotation_y(self.camera_yaw.to_radians());
            let pitch_rotation = Quat::from_rotation_x(self.camera_pitch.to_radians());
            pitch_rotation * yaw_rotation
        };

        self.camera.rotation = rotation;

        self.camera.position += self.camera.forward() * self.moving_forward;
        self.camera.position += self.camera.right() * self.moving_right;
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
