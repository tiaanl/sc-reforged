use glam::{Mat4, Quat, Vec3};
use tracing::warn;

use crate::engine::{
    arena::{Arena, Handle},
    mesh::{GpuMesh, RenderPassMeshExt},
    renderer::{RenderPipelineConfig, Renderer},
};

use super::textures::Textures;

pub struct ModelMesh {
    pub mesh: GpuMesh,
    pub texture_handle: Handle<wgpu::BindGroup>,
}

pub struct ModelNode {
    pub position: Vec3,
    pub rotation: Quat,
    pub meshes: Vec<ModelMesh>,
    pub children: Vec<ModelNode>,
}

#[derive(Default)]
pub struct Model {
    pub nodes: Vec<ModelNode>,
}

pub struct RenderInfo {
    position: Vec3,
    rotation: Vec3,
    handle: Handle<Model>,
}

impl RenderInfo {
    pub fn new(position: Vec3, rotation: Vec3, handle: Handle<Model>) -> Self {
        Self {
            position,
            rotation,
            handle,
        }
    }
}

pub struct Models {
    render_pipeline: wgpu::RenderPipeline,
    models: Arena<Model>,
}

impl Models {
    pub fn new(renderer: &Renderer) -> Self {
        let shader_module =
            renderer.create_shader_module("model_renderer", include_str!("model.wgsl"));

        let render_pipeline = renderer.create_render_pipeline(
            RenderPipelineConfig::<crate::engine::mesh::Vertex>::new(
                "model_renderer",
                &shader_module,
            )
            .bind_group_layout(renderer.uniform_bind_group_layout())
            .bind_group_layout(renderer.uniform_bind_group_layout())
            .bind_group_layout(renderer.texture_bind_group_layout()),
        );

        let models = Arena::default();

        Self {
            render_pipeline,
            models,
        }
    }

    pub fn insert(&mut self, model: Model) -> Handle<Model> {
        self.models.insert(model)
    }

    pub fn render_multiple(
        &self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        textures: &Textures,
        camera_bind_group: &wgpu::BindGroup,
        batch: &[RenderInfo],
    ) {
        let mut render_pass = Self::create_render_pass(renderer, encoder, output);
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        batch.iter().for_each(|render_info| {
            let Some(model) = self.models.get(&render_info.handle) else {
                return;
            };

            let rotation = Mat4::from_rotation_x(render_info.rotation.x)
                * Mat4::from_rotation_y(render_info.rotation.y)
                * Mat4::from_rotation_z(render_info.rotation.z);
            let model_matrix = Mat4::from_translation(render_info.position) * rotation;
            let data = model_matrix.to_cols_array_2d();
            let buffer = renderer.create_uniform_buffer("model_matrix_buffer", data);
            let model_bind_group = renderer.create_uniform_bind_group("model_matrix", &buffer);

            render_pass.set_bind_group(1, &model_bind_group, &[]);
            model.nodes.iter().for_each(|node| {
                self.render_node(&mut render_pass, textures, node);
            });
        });
    }

    fn render_node(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        textures: &Textures,
        node: &ModelNode,
    ) {
        node.meshes.iter().for_each(|mesh| {
            if let Some(texture_bind_group) = textures.get(&mesh.texture_handle) {
                render_pass.set_bind_group(2, texture_bind_group, &[]);
                render_pass.draw_mesh(&mesh.mesh);
            } else {
                warn!("Could not find model texture!");
            }
        });

        for node in node.children.iter() {
            self.render_node(render_pass, textures, node)
        }
    }

    fn create_render_pass<'encoder>(
        renderer: &Renderer,
        encoder: &'encoder mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
    ) -> wgpu::RenderPass<'encoder> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("model_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output,
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
        })
    }
}
