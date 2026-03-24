use glam::UVec2;

use crate::engine::renderer::RenderContext;

use super::GeometryBuffer;

pub struct RenderTargets {
    pub surface_size: UVec2,
    pub surface_format: wgpu::TextureFormat,
    pub geometry_buffer: GeometryBuffer,
}

impl RenderTargets {
    pub fn new(
        context: &RenderContext,
        surface_size: UVec2,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let geometry_buffer = GeometryBuffer::new(&context.device, surface_size);

        Self {
            surface_size,
            surface_format,
            geometry_buffer,
        }
    }

    pub fn resize(&mut self, context: &RenderContext, size: UVec2) {
        self.surface_size = size;
        self.geometry_buffer.resize(&context.device, size);
    }
}
