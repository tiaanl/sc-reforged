use crate::engine::renderer::Gpu;

pub struct UniformBuffer {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl UniformBuffer {
    pub fn new(buffer: wgpu::Buffer, bind_group: wgpu::BindGroup) -> Self {
        Self { buffer, bind_group }
    }

    #[inline]
    pub fn write(&self, gpu: &Gpu, data: &[u8]) {
        gpu.queue.write_buffer(&self.buffer, 0, data)
    }
}
