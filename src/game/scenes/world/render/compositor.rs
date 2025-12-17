use crate::{engine::renderer::Frame, game::scenes::world::render::GeometryBuffer, wgsl_shader};

pub struct Compositor {
    pipeline: wgpu::RenderPipeline,
}

impl Compositor {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        geometry_buffer: &GeometryBuffer,
    ) -> Self {
        let module = device.create_shader_module(wgsl_shader!("compositor"));

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("compositor_pipeline_layout"),
            bind_group_layouts: &[&geometry_buffer.bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("compositor_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        Self { pipeline }
    }

    pub fn render(&self, frame: &mut Frame, geometry_buffer: &GeometryBuffer) {
        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("compositor_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.surface,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, geometry_buffer.bind_group(), &[]);
        render_pass.draw(0..3, 0..1);
    }
}
