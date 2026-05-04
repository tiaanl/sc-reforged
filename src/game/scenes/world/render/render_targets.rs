use glam::UVec2;

use crate::engine::renderer::Gpu;

use super::GeometryBuffer;

pub struct RenderTargets {
    pub surface_size: UVec2,
    pub surface_format: wgpu::TextureFormat,
    pub geometry_buffer: GeometryBuffer,
}

impl RenderTargets {
    pub fn new(gpu: &Gpu, surface_size: UVec2, surface_format: wgpu::TextureFormat) -> Self {
        let geometry_buffer = GeometryBuffer::new(&gpu.device, surface_size);

        Self {
            surface_size,
            surface_format,
            geometry_buffer,
        }
    }

    pub fn resize(&mut self, gpu: &Gpu, size: UVec2) {
        self.surface_size = size;
        self.geometry_buffer.resize(&gpu.device, size);
    }
}
