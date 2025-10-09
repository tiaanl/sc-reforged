use std::ops::Range;

use ahash::HashMap;

use super::render_textures::RenderTextures;
use crate::{
    engine::{
        assets::AssetError,
        growing_buffer::GrowingBuffer,
        mesh::IndexedMesh,
        prelude::renderer,
        storage::{Handle, Storage},
    },
    game::{
        image::BlendMode,
        math::BoundingSphere,
        model::Model,
        models::models,
        renderer::render_animations::{RenderAnimation, RenderAnimations},
    },
};

#[derive(Clone, Copy, Debug, bytemuck::NoUninit)]
#[repr(C)]
pub struct RenderVertex {
    pub position: [f32; 4],
    pub normal: [f32; 4],
    pub tex_coord: [f32; 2],
    pub node_index: u32,
    pub texture_data_index: u32,
}

pub struct RenderModel {
    /// Range for opaque mesh indices.
    pub opaque_range: Range<u32>,
    /// Range for alpha blended mesh indices.
    pub alpha_range: Range<u32>,
    /// Range dor additive blended mesh indices.
    pub additive_range: Range<u32>,
    /// A [BoundingSphere] that wraps the entire model. Used for culling.
    pub bounding_sphere: BoundingSphere,
    /// A [RenderAnimation] with a single frame that represents the model at rest.
    pub rest_pose: Handle<RenderAnimation>,
}

pub struct RenderModels {
    /// Buffer containing all model vertices.
    vertices_buffer: GrowingBuffer<RenderVertex>,
    /// Buffer containing all model vertices.
    indices_buffer: GrowingBuffer<u32>,
    /// Local data for each model.
    models: Storage<RenderModel>,
    /// Cache of model handles to render model handles.
    model_to_render_model: HashMap<Handle<Model>, Handle<RenderModel>>,
}

impl RenderModels {
    const INITIAL_VERTEX_COUNT: u32 = 4_096;
    const INITIAL_INDEX_COUNT: u32 = 4_096;

    pub fn new() -> Self {
        let vertices_buffer = GrowingBuffer::new(
            renderer(),
            Self::INITIAL_VERTEX_COUNT,
            wgpu::BufferUsages::VERTEX,
            "render_models_vertices",
        );

        let indices_buffer = GrowingBuffer::new(
            renderer(),
            Self::INITIAL_INDEX_COUNT,
            wgpu::BufferUsages::INDEX,
            "render_models_indices",
        );

        let models = Storage::default();

        let model_to_render_model = HashMap::default();

        Self {
            vertices_buffer,
            indices_buffer,
            models,
            model_to_render_model,
        }
    }

    pub fn vertices_buffer_slice(&self) -> wgpu::BufferSlice<'_> {
        self.vertices_buffer.slice(..)
    }

    pub fn indices_buffer_slice(&self) -> wgpu::BufferSlice<'_> {
        self.indices_buffer.slice(..)
    }

    pub fn get_or_create(
        &mut self,
        render_textures: &mut RenderTextures,
        animations: &mut RenderAnimations,
        model_handle: Handle<Model>,
    ) -> Result<Handle<RenderModel>, AssetError> {
        if let Some(render_model_handle) = self.model_to_render_model.get(&model_handle) {
            return Ok(*render_model_handle);
        }

        let model = models()
            .get(model_handle)
            .expect("Model should have been loaded byt his time.");

        let mut opaque_mesh = IndexedMesh::default();
        let mut alpha_mesh = IndexedMesh::default();
        let mut additive_mesh = IndexedMesh::default();

        for mesh in model.meshes.iter() {
            let texture_handle = render_textures.get_or_create(mesh.image);
            let texture = render_textures.get(texture_handle).unwrap();

            let indexed_mesh = match texture.blend_mode {
                BlendMode::Opaque | BlendMode::ColorKeyed => &mut opaque_mesh,
                BlendMode::Alpha => &mut alpha_mesh,
                BlendMode::Additive => &mut additive_mesh,
            };

            // Extend the mesh for the texture with the data from the model.
            indexed_mesh.extend(IndexedMesh {
                vertices: mesh
                    .mesh
                    .vertices
                    .iter()
                    .map(|v| RenderVertex {
                        position: v.position.extend(1.0).to_array(),
                        normal: v.normal.extend(0.0).to_array(),
                        tex_coord: v.tex_coord.to_array(),
                        node_index: v.node_index,
                        texture_data_index: texture.texture_data_index,
                    })
                    .collect(),
                indices: mesh.mesh.indices.clone(),
            });
        }

        let mut push_mesh = |mut indexed_mesh: IndexedMesh<RenderVertex>| {
            let vertices_range = self
                .vertices_buffer
                .extend(renderer(), &indexed_mesh.vertices);
            // Adjust the indices to point to the range of the vertices.
            indexed_mesh
                .indices
                .iter_mut()
                .for_each(|i| *i += vertices_range.start);

            self.indices_buffer
                .extend(renderer(), &indexed_mesh.indices)
        };

        let opaque_range = push_mesh(opaque_mesh);
        let alpha_range = push_mesh(alpha_mesh);
        let additive_range = push_mesh(additive_mesh);

        let rest_pose = animations.create_rest_pose(&model.skeleton);

        let render_model = self.models.insert(RenderModel {
            opaque_range,
            alpha_range,
            additive_range,
            bounding_sphere: model.bounding_sphere,
            rest_pose,
        });

        self.model_to_render_model
            .insert(model_handle, render_model);

        Ok(render_model)
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
