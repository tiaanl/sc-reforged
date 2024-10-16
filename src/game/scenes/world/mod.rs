use crate::{
    engine::{
        assets::{AssetError, Assets},
        renderer::Renderer,
        scene::Scene,
    },
    game::config::CampaignDef,
};
use camera::*;
use glam::{Mat4, Quat, Vec3};
use terrain::*;
use wgpu::util::DeviceExt;

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

    camera_diff: Vec3,
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

        let camera = Camera::from_position_rotation(
            Vec3::new(1000.0, 2000.0, -3000.0),
            Quat::from_rotation_x(-15.0_f32.to_radians())
                * Quat::from_rotation_y(-30.0_f32.to_radians()),
        );

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

            camera_diff: Vec3::ZERO,
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

    fn update(&mut self, delta_time: f32) {
        // self.camera.position += self.camera_diff;
        self.terrain.update(delta_time);

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
