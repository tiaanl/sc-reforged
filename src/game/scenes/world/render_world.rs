use bytemuck::NoUninit;

use crate::engine::prelude::Renderer;

use super::sim_world::SimWorld;

#[derive(Clone, Copy, Default, NoUninit)]
#[repr(C)]
pub struct Camera {
    pub proj_view: [[f32; 4]; 4],
    pub frustum: [[f32; 4]; 6],
    pub position: [f32; 4], // x, y, z, 1
    pub forward: [f32; 4],  // x, y, z, 0
}

#[derive(Clone, Copy, Default, NoUninit)]
#[repr(C)]
pub struct Environment {
    pub sun_dir: [f32; 4],   // x, y, z, 0
    pub sun_color: [f32; 4], // r, g, b, 1
    pub fog_color: [f32; 4], // r, g, b, 1
    pub fog_distance: f32,
    pub fog_near_fraction: f32,
    pub _pad: [f32; 2],
}

pub struct RenderWorld {
    pub cameras: [Camera; SimWorld::CAMERA_COUNT],
    pub environment: Environment,

    pub camera_buffer: wgpu::Buffer,
    pub environment_buffer: wgpu::Buffer,
}

impl RenderWorld {
    pub fn new(renderer: &Renderer) -> Self {
        let camera_buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cameras"),
            size: std::mem::size_of::<[Camera; SimWorld::CAMERA_COUNT]>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let environment_buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("environment"),
            size: std::mem::size_of::<Environment>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            cameras: [Camera::default(), Camera::default()],
            environment: Environment::default(),
            camera_buffer,
            environment_buffer,
        }
    }
}
