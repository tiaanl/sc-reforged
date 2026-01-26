use glam::UVec2;

use crate::engine::renderer::Renderer;

use super::{GeometryBuffer, RenderWorld};

pub struct RenderStore {
    pub surface_size: UVec2,

    pub geometry_buffer: GeometryBuffer,

    pub camera_bind_group_layout: wgpu::BindGroupLayout,
    pub ui_state_bind_group_layout: wgpu::BindGroupLayout,
}

impl RenderStore {
    pub fn new(renderer: &Renderer) -> Self {
        let geometry_buffer = GeometryBuffer::new(&renderer.device, UVec2::ZERO);

        let camera_bind_group_layout = RenderWorld::create_camera_bind_group_layout(renderer);
        let ui_state_bind_group_layout = RenderWorld::create_ui_state_bind_group_layout(renderer);

        Self {
            surface_size: UVec2::ZERO,

            geometry_buffer,

            camera_bind_group_layout,
            ui_state_bind_group_layout,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: UVec2) {
        self.surface_size = size;
        self.geometry_buffer.resize(device, size);
    }
}
