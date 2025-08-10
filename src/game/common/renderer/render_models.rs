use std::ops::Range;

use glam::{Vec2, Vec3};
use wgpu::util::DeviceExt;

use super::render_textures::{RenderTexture, RenderTextures};
use crate::{
    engine::{
        assets::AssetError,
        prelude::renderer,
        storage::{Handle, Storage},
    },
    game::{
        image::BlendMode,
        model,
        models::models,
        renderer::{
            InstanceKey,
            render_animations::{RenderAnimation, RenderAnimations},
        },
    },
};

type NodeIndex = u32;

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
pub struct RenderVertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub tex_coord: Vec2,
    pub node_index: NodeIndex,
}

struct RenderMesh {
    indices: Range<u32>,
    texture_handle: Handle<RenderTexture>,
}

pub struct RenderModel {
    /// Contains the vertices for the entire model.
    vertex_buffer: wgpu::Buffer,
    /// Contains the indices for the entire model.
    index_buffer: wgpu::Buffer,
    /// All the meshes (sets of indices) that the model consists of.
    meshes: Vec<RenderMesh>,
    /// A [RenderAnimation] with a single frame that represents the model at rest.
    pub rest_pose: Handle<RenderAnimation>,
}

impl RenderModel {
    pub fn render(
        &self,
        render_pass: &mut wgpu::RenderPass,
        textures: &RenderTextures,
        animations: &RenderAnimations,
        blend_mode: BlendMode,
    ) {
        for mesh in self.meshes.iter() {
            let Some(texture) = textures.get(mesh.texture_handle) else {
                tracing::warn!("Texture not in cache");
                continue;
            };

            if texture.blend_mode != blend_mode {
                continue;
            }

            render_pass.set_bind_group(2, &texture.bind_group, &[]);

            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(mesh.indices.clone(), 0, 0..1);
        }
    }
}

#[derive(Default)]
pub struct RenderModels {
    models: Storage<RenderModel>,
}

impl RenderModels {
    pub fn add_model(
        &mut self,
        textures: &mut RenderTextures,
        animations: &mut RenderAnimations,
        model_handle: Handle<model::Model>,
    ) -> Result<InstanceKey, AssetError> {
        let model = models()
            .get(model_handle)
            .expect("Model should have been loaded byt his time.");

        let mut meshes = Vec::default();

        let mut vertices = Vec::default();
        let mut indices = Vec::default();

        let mut first_vertex_index = 0;

        for mesh in model.meshes.iter() {
            let texture_handle = textures.add(mesh.image);

            mesh.mesh
                .vertices
                .iter()
                .map(|v| RenderVertex {
                    position: v.position,
                    normal: v.normal,
                    tex_coord: v.tex_coord,
                    node_index: v.node_index,
                })
                .for_each(|v| vertices.push(v));

            let first_index = indices.len() as u32;

            mesh.mesh
                .indices
                .iter()
                .map(|index| index + first_vertex_index)
                .for_each(|i| indices.push(i));

            let last_index = indices.len() as u32;

            meshes.push(RenderMesh {
                indices: first_index..last_index,
                texture_handle,
            });

            first_vertex_index = vertices.len() as u32;
        }

        let vertex_buffer =
            renderer()
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("model_vertex_buffer"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

        let index_buffer =
            renderer()
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("model_index_buffer"),
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

        let rest_pose = animations.create_rest_pose(&model.nodes);

        let render_model_handle = self.models.insert(RenderModel {
            vertex_buffer,
            index_buffer,
            meshes,
            rest_pose,
        });

        Ok(InstanceKey::new(render_model_handle, None))
    }

    #[inline]
    pub fn get(&self, handle: Handle<RenderModel>) -> Option<&RenderModel> {
        self.models.get(handle)
    }

    #[inline]
    pub fn get_mut(&mut self, handle: Handle<RenderModel>) -> Option<&mut RenderModel> {
        self.models.get_mut(handle)
    }
}
