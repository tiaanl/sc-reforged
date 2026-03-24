#[derive(Clone)]
pub struct RenderContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl RenderContext {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        Self { device, queue }
    }
}
