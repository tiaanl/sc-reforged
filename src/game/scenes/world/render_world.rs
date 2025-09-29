use bytemuck::NoUninit;

use crate::engine::prelude::Renderer;

use super::sim_world::SimWorld;

#[derive(Clone, Copy, Default, NoUninit)]
#[repr(C)]
pub struct Camera {
    pub proj_view: [[f32; 4]; 4],
    pub frustum: [[f32; 4]; 6],
    pub position: [f32; 4],
    pub forward: [f32; 4],
}

pub struct RenderWorld {
    pub cameras: [Camera; SimWorld::CAMERA_COUNT],

    pub camera_buffer: wgpu::Buffer,
}

impl RenderWorld {
    pub fn new(renderer: &Renderer) -> Self {
        let camera_buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cameras"),
            size: std::mem::size_of::<[Camera; SimWorld::CAMERA_COUNT]>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            cameras: [Camera::default(), Camera::default()],
            camera_buffer,
        }
    }
}
