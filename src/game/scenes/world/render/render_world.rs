use crate::{
    engine::renderer::Renderer,
    game::scenes::world::render::{
        camera_render_pipeline::{self, CameraEnvironmentLayout},
        render_layouts::RenderLayouts,
        ui_render_pipeline::{self, UiStateLayout},
        uniform_buffer::UniformBuffer,
    },
};

/// Set of data that changes on each frame.
pub struct RenderWorld {
    pub camera_env_buffer: UniformBuffer,
    pub ui_state_buffer: UniformBuffer,
}

impl RenderWorld {
    pub fn new(index: usize, renderer: &Renderer, layouts: &mut RenderLayouts) -> Self {
        let camera_env_buffer = {
            let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("cameras"),
                size: std::mem::size_of::<camera_render_pipeline::gpu::CameraEnvironment>()
                    as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let bind_group = renderer
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(&format!("cmaera_bind_group_{index}")),
                    layout: layouts.get::<CameraEnvironmentLayout>(renderer),
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                });

            UniformBuffer::new(buffer, bind_group)
        };

        let ui_state_buffer = {
            let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ui_state_buffer"),
                size: std::mem::size_of::<ui_render_pipeline::gpu::State>() as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
                mapped_at_creation: false,
            });

            let bind_group = renderer
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(&format!("ui_state_bind_group_{index}")),
                    layout: layouts.get::<UiStateLayout>(renderer),
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                });

            UniformBuffer::new(buffer, bind_group)
        };

        Self {
            camera_env_buffer,
            ui_state_buffer,
        }
    }
}
