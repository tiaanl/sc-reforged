use glam::Vec3;

use crate::engine::{
    mesh::{GpuMesh, RenderPassMeshExt},
    renderer::{GpuTexture, RenderPipelineConfig, Renderer},
};

pub struct ModelRenderer {
    render_pipeline: wgpu::RenderPipeline,
}

impl ModelRenderer {
    pub fn new(renderer: &Renderer, camera_bind_group_layout: &wgpu::BindGroupLayout) -> Self {
        let shader_module =
            renderer.create_shader_module("model_renderer", include_str!("model.wgsl"));

        let render_pipeline = renderer.create_render_pipeline(
            RenderPipelineConfig::<crate::engine::mesh::Vertex>::new(
                "model_renderer",
                &shader_module,
            )
            .bind_group_layout(camera_bind_group_layout)
            .bind_group_layout(renderer.texture_bind_group_layout()),
        );

        Self { render_pipeline }
    }

    pub fn render(
        &self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        camera_bind_group: &wgpu::BindGroup,
        mesh: &GpuMesh,
        texture: &GpuTexture,
        _position: Vec3,
    ) {
        let texture_bind_group = renderer.create_texture_bind_group(
            "texture_bind_group",
            &texture.view,
            &texture.sampler,
        );

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("model_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: renderer
                .render_pass_depth_stencil_attachment(wgpu::LoadOp::Load),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, &texture_bind_group, &[]);
        render_pass.draw_mesh(mesh);
    }
}
