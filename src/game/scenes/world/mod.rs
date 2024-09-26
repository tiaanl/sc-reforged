use crate::{
    engine::{assets::Assets, renderer::Renderer, scene::Scene},
    game::config::CampaignDef,
};
use camera::*;
use cgmath::{prelude::*, vec3, Deg, Quaternion, Vector3};
use terrain::*;
use wgpu::util::DeviceExt;

mod camera;
mod terrain;

/// The [Scene] that renders the ingame world view.

pub struct WorldScene {
    campaign_def: CampaignDef,

    camera: Camera,
    camera_bind_group_layout: wgpu::BindGroupLayout,

    terrain: Terrain,

    camera_diff: Vector3<f32>,
}

impl WorldScene {
    pub fn new(_assets: &Assets, renderer: &Renderer, campaign_def: CampaignDef) -> Self {
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

        let camera = Camera::from_position_rotation(
            Vector3::new(0.0, 0.0, 5.0),
            Quaternion::from_angle_y(Deg(0.0)),
        );

        let terrain = Terrain::new(renderer, &camera_bind_group_layout);

        Self {
            campaign_def,
            camera,
            terrain,

            camera_bind_group_layout,

            camera_diff: Vector3::new(0.0, 0.001, 0.0),
        }
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

    fn update(&mut self, _delta_time: f32) {
        self.camera.position += self.camera_diff;

        // self.camera.position = vec3(0.0, 1.0, -5.0);
        // self.camera.look_at(vec3(0.0, 0.0, 0.0));
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
