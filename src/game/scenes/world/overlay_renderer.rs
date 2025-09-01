use crate::{
    engine::{prelude::Frame, renderer::renderer},
    game::{geometry_buffers::GeometryBuffers, shadows::ShadowCascades},
    wgsl_shader,
};

pub struct OverlayRenderer {
    pipeline: wgpu::RenderPipeline,
}

impl OverlayRenderer {
    pub fn new(
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        shadow_cascades: &ShadowCascades,
        geometry_buffers: &GeometryBuffers,
    ) -> Self {
        let module = renderer()
            .device
            .create_shader_module(wgsl_shader!("overlay"));

        let layout = renderer()
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("overlay"),
                bind_group_layouts: &[
                    camera_bind_group_layout,
                    &shadow_cascades.cascades_bind_group_layout,
                    &geometry_buffers.bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let pipeline = renderer()
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("overlay"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vertex_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fragment_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: renderer().surface.format(),
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            });

        Self { pipeline }
    }

    pub fn render(
        &mut self,
        frame: &mut Frame,
        camera_bind_group: &wgpu::BindGroup,
        shadow_cascades: &ShadowCascades,
        geometry_buffers: &GeometryBuffers,
    ) {
        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("overlay"),
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
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, &shadow_cascades.cascades_bind_group, &[]);
        render_pass.set_bind_group(2, &geometry_buffers.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
