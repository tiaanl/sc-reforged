use crate::game::globals;

pub struct UniformBuffer {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl UniformBuffer {
    pub fn new(buffer: wgpu::Buffer, bind_group: wgpu::BindGroup) -> Self {
        Self { buffer, bind_group }
    }

    #[inline]
    pub fn write(&self, data: &[u8]) {
        globals::gpu().queue.write_buffer(&self.buffer, 0, data)
    }
}
