use crate::engine::{
    renderer::{Gpu, RenderContext, RenderTarget},
    shader_cache::{ShaderCache, ShaderSource},
};

pub struct Compositor {
    pipeline: wgpu::RenderPipeline,
}

impl Compositor {
    pub fn new(
        gpu: &Gpu,
        target_format: wgpu::TextureFormat,
        gbuffer_bind_group_layout: &wgpu::BindGroupLayout,
        shader_cache: &mut ShaderCache,
    ) -> Self {
        let module = shader_cache.get_or_create(&gpu.device, ShaderSource::Compositor);

        let layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("compositor_pipeline_layout"),
                bind_group_layouts: &[gbuffer_bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("compositor_pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module,
                    entry_point: Some("vertex"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module,
                    entry_point: Some("fragment"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target_format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            });

        Self { pipeline }
    }
}

impl Compositor {
    pub fn composite(
        &self,
        render_context: &mut RenderContext,
        render_target: &RenderTarget,
        gbuffer_bind_group: &wgpu::BindGroup,
    ) {
        let mut render_pass =
            render_context
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("compositor_render_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &render_target.view,
                        depth_slice: None,
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
        render_pass.set_bind_group(0, gbuffer_bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
