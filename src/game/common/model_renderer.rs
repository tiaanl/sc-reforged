use crate::engine::prelude::*;

use super::{
    geometry_buffers::GeometryBuffers,
    model::{Model, ModelVertex},
    render::RenderTexture,
};

pub struct ModelRenderer {
    pipeline: wgpu::RenderPipeline,
}

impl ModelRenderer {
    pub fn new(
        renderer: &Renderer,
        shaders: &mut Shaders,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let module = shaders.create_shader(
            renderer,
            "model_renderer_shader",
            include_str!("model.wgsl"),
            "model.wgsl",
            Default::default(),
        );

        let layout = renderer
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("model_renderer_pipeline_layout"),
                bind_group_layouts: &[
                    camera_bind_group_layout,
                    renderer.texture_bind_group_layout(),
                ],
                push_constant_ranges: &[],
            });

        let pipeline = renderer
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("model_renderer_pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[ModelVertex::layout()],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(
                    renderer
                        .depth_buffer
                        .depth_stencil_state(wgpu::CompareFunction::LessEqual, true),
                ),
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: GeometryBuffers::targets(),
                }),
                multiview: None,
                cache: None,
            });

        Self { pipeline }
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        geometry_buffers: &GeometryBuffers,
        camera_bind_group: &wgpu::BindGroup,
        _models: &[&Model],
        _texture_storage: &Storage<RenderTexture>,
    ) {
        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("model_renderer_render_pass"),
                color_attachments: &geometry_buffers.color_attachments(),
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &frame.depth_buffer.texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);

        // for model in models.iter() {
        //     for mesh in model.meshes.iter() {
        //         let Some(texture) = texture_storage.get(mesh.texture) else {
        //             continue;
        //         };

        //         render_pass.set_bind_group(1, &texture.bind_group, &[]);
        //         render_pass.set_vertex_buffer(0, mesh.mesh.vertex_buffer.slice(..));
        //         render_pass
        //             .set_index_buffer(mesh.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        //         render_pass.draw_indexed(0..mesh.mesh.index_count, 0, 0..1);
        //     }
        // }
    }
}
