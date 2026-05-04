use crate::{
    engine::renderer::Gpu,
    game::render::{
        per_frame::PerFrame, uniform_buffer::UniformBuffer, world::render_layouts::RenderLayouts,
    },
};

/// Set of data that changes on each frame.
pub struct RenderBindings {
    pub camera_env_buffer: PerFrame<UniformBuffer>,
}

impl RenderBindings {
    pub fn new(gpu: &Gpu, layouts: &mut RenderLayouts) -> Self {
        let layout = layouts.get::<super::camera_render_pipeline::CameraEnvironmentLayout>();

        let camera_env_buffer = PerFrame::new(|index| {
            let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("cameras"),
                size: std::mem::size_of::<super::camera_render_pipeline::gpu::CameraEnvironment>()
                    as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("cmaera_bind_group_{index}")),
                layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });

            UniformBuffer::new(buffer, bind_group)
        });

        Self { camera_env_buffer }
    }
}
