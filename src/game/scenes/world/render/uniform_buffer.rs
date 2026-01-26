use crate::engine::renderer::Renderer;

pub struct UniformBuffer {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl UniformBuffer {
    pub fn new(buffer: wgpu::Buffer, bind_group: wgpu::BindGroup) -> Self {
        Self { buffer, bind_group }
    }

    #[inline]
    pub fn write(&self, renderer: &Renderer, data: &[u8]) {
        renderer.queue.write_buffer(&self.buffer, 0, data)
    }
}
