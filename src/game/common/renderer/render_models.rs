use ahash::HashMap;
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
        image::Image,
        model::{self, Model},
        models::models,
        renderer::{
            render_animations::{RenderAnimation, RenderAnimations},
            render_textures::RenderTextureSet,
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
    pub texture_index: u32,
}

pub struct RenderModel {
    /// Contains the vertices for the entire model.
    pub vertex_buffer: wgpu::Buffer,
    /// Contains the indices for the entire model.
    pub index_buffer: wgpu::Buffer,
    /// The total number of indices in the mesh.
    pub index_count: u32,
    /// All the textures used by the model.
    pub texture_set: RenderTextureSet,
    /// A [RenderAnimation] with a single frame that represents the model at rest.
    pub rest_pose: Handle<RenderAnimation>,
}

#[derive(Default)]
pub struct RenderModels {
    models: Storage<RenderModel>,

    model_to_render_model: HashMap<Handle<Model>, Handle<RenderModel>>,
}

impl RenderModels {
    pub fn add_model(
        &mut self,
        render_textures: &mut RenderTextures,
        animations: &mut RenderAnimations,
        model_handle: Handle<model::Model>,
    ) -> Result<Handle<RenderModel>, AssetError> {
        if let Some(render_model_handle) = self.model_to_render_model.get(&model_handle) {
            return Ok(*render_model_handle);
        }

        let model = models()
            .get(model_handle)
            .expect("Model should have been loaded byt his time.");

        let mut textures: Vec<Handle<RenderTexture>> = Vec::with_capacity(8);

        let mut vertices = Vec::default();
        let mut indices = Vec::default();

        let mut first_vertex_index = 0;

        let mut image_to_index: HashMap<Handle<Image>, u32> = HashMap::default();

        for mesh in model.meshes.iter() {
            let texture_index = {
                if let Some(texture_index) = image_to_index.get(&mesh.image) {
                    *texture_index
                } else {
                    let texture_handle = render_textures.add(mesh.image);
                    let texture_index = textures.len() as u32;
                    textures.push(texture_handle);

                    image_to_index.insert(mesh.image, texture_index);

                    texture_index
                }
            };

            mesh.mesh
                .vertices
                .iter()
                .map(|v| RenderVertex {
                    position: v.position,
                    normal: v.normal,
                    tex_coord: v.tex_coord,
                    node_index: v.node_index,
                    texture_index,
                })
                .for_each(|v| vertices.push(v));

            mesh.mesh
                .indices
                .iter()
                .map(|index| index + first_vertex_index)
                .for_each(|i| indices.push(i));

            first_vertex_index = vertices.len() as u32;
        }

        #[cfg(debug_assertions)]
        if vertices.is_empty() || indices.is_empty() {
            unreachable!("Mesh with no vertices or indices! Should be checked up-front!");
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

        let texture_set = render_textures.create_texture_set(textures);

        let rest_pose = animations.create_rest_pose(&model.skeleton);

        let render_model_handle = self.models.insert(RenderModel {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            texture_set,
            rest_pose,
        });

        self.model_to_render_model
            .insert(model_handle, render_model_handle);

        Ok(render_model_handle)
    }

    #[inline]
    pub fn get(&self, handle: Handle<RenderModel>) -> Option<&RenderModel> {
        self.models.get(handle)
    }

    #[inline]
    pub fn _get_mut(&mut self, handle: Handle<RenderModel>) -> Option<&mut RenderModel> {
        self.models.get_mut(handle)
    }
}
