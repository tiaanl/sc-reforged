use glam::UVec2;

use crate::engine::renderer::Renderer;

use super::GeometryBuffer;

pub struct RenderTargets {
    pub surface_size: UVec2,
    pub surface_format: wgpu::TextureFormat,
    pub geometry_buffer: GeometryBuffer,
}

impl RenderTargets {
    pub fn new(
        renderer: &Renderer,
        surface_size: UVec2,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let geometry_buffer = GeometryBuffer::new(&renderer.device, surface_size);

        Self {
            surface_size,
            surface_format,
            geometry_buffer,
        }
    }

    pub fn resize(&mut self, renderer: &Renderer, size: UVec2) {
        self.surface_size = size;
        self.geometry_buffer.resize(&renderer.device, size);
    }
}
