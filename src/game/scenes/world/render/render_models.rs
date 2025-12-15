use std::ops::Range;

use super::render_textures::RenderTextures;
use crate::{
    engine::{
        assets::AssetError,
        growing_buffer::GrowingBuffer,
        mesh::IndexedMesh,
        renderer::Renderer,
        storage::{Handle, Storage},
    },
    game::{image::BlendMode, model::Model, models::models},
};

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
pub struct RenderNode {
    transform: [[f32; 4]; 4],
    parent_index: u32,
    _pad: [u32; 3],
}

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
pub struct RenderVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
    pub node_index: u32,
    pub texture_data_index: u32,
}

pub struct RenderModel {
    /// Range for opaque mesh indices.
    pub opaque_range: Range<u32>,
    /// Range for alpha blended mesh indices.
    pub alpha_range: Range<u32>,
    /// Range for additive blended mesh indices.
    pub _additive_range: Range<u32>,
    /// Range for [RenderNode]'s for the model.
    pub nodes_range: Range<u32>,
}

pub struct RenderModels {
    /// Buffer containing all model vertices.
    vertices_buffer: GrowingBuffer<RenderVertex>,
    /// Buffer containing all model vertices.
    indices_buffer: GrowingBuffer<u32>,
    /// Buffer containing node data for all models.
    nodes_buffer: GrowingBuffer<RenderNode>,
    /// Bind group layout for the [RenderNode]'s.
    pub nodes_bind_group_layout: wgpu::BindGroupLayout,
    /// Bind group for the [RenderNode]'s.
    pub nodes_bind_group: wgpu::BindGroup,
    /// Local data for each model.
    models: Storage<RenderModel>,
}

impl RenderModels {
    const INITIAL_VERTEX_COUNT: u32 = 1 << 15;
    const INITIAL_INDEX_COUNT: u32 = 1 << 15;
    const INITIAL_NODES_COUNT: u32 = 1 << 15;

    pub fn new(renderer: &Renderer) -> Self {
        let vertices_buffer = GrowingBuffer::new(
            renderer,
            Self::INITIAL_VERTEX_COUNT,
            wgpu::BufferUsages::VERTEX,
            "render_models_vertices",
        );

        let indices_buffer = GrowingBuffer::new(
            renderer,
            Self::INITIAL_INDEX_COUNT,
            wgpu::BufferUsages::INDEX,
            "render_models_indices",
        );

        let nodes_buffer = GrowingBuffer::new(
            renderer,
            Self::INITIAL_NODES_COUNT,
            wgpu::BufferUsages::STORAGE,
            "render_models_nodes",
        );

        let nodes_bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("model_nodes_bind_group_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let nodes_bind_group = Self::create_nodes_bind_group(
            &renderer.device,
            &nodes_bind_group_layout,
            nodes_buffer.buffer(),
        );

        let models = Storage::default();

        Self {
            vertices_buffer,
            indices_buffer,
            nodes_buffer,
            nodes_bind_group_layout,
            nodes_bind_group,
            models,
        }
    }

    pub fn vertices_buffer_slice(&self) -> wgpu::BufferSlice<'_> {
        self.vertices_buffer.slice(..)
    }

    pub fn indices_buffer_slice(&self) -> wgpu::BufferSlice<'_> {
        self.indices_buffer.slice(..)
    }

    pub fn add(
        &mut self,
        renderer: &Renderer,
        render_textures: &mut RenderTextures,
        model_handle: Handle<Model>,
    ) -> Result<Handle<RenderModel>, AssetError> {
        let model = models()
            .get(model_handle)
            .expect("Model should have been loaded byt his time.");

        let mut opaque_mesh = IndexedMesh::default();
        let mut alpha_mesh = IndexedMesh::default();
        let mut additive_mesh = IndexedMesh::default();

        let nodes: Vec<RenderNode> = model
            .skeleton
            .bones
            .iter()
            .map(|bone| RenderNode {
                transform: bone.transform.to_mat4().to_cols_array_2d(),
                parent_index: bone.parent,
                _pad: Default::default(),
            })
            .collect();

        for mesh in model.meshes.iter() {
            let texture_handle = render_textures.get_or_create(renderer, mesh.image);
            let texture = render_textures.get(texture_handle).unwrap();

            let indexed_mesh = match texture.blend_mode {
                BlendMode::Opaque | BlendMode::ColorKeyed => &mut opaque_mesh,
                BlendMode::Alpha => &mut alpha_mesh,
                BlendMode::_Additive => &mut additive_mesh,
            };

            // Extend the mesh for the texture with the data from the model.
            indexed_mesh.extend(IndexedMesh {
                vertices: mesh
                    .mesh
                    .vertices
                    .iter()
                    .map(|v| RenderVertex {
                        position: v.position.to_array(),
                        normal: v.normal.to_array(),
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
                .extend(renderer, &indexed_mesh.vertices);
            // Adjust the indices to point to the range of the vertices.
            indexed_mesh
                .indices
                .iter_mut()
                .for_each(|i| *i += vertices_range.start);

            self.indices_buffer.extend(renderer, &indexed_mesh.indices)
        };

        let opaque_range = push_mesh(opaque_mesh);
        let alpha_range = push_mesh(alpha_mesh);
        let additive_range = push_mesh(additive_mesh);

        let nodes_range = {
            let old_capacity = self.nodes_buffer.capacity;
            let range = self.nodes_buffer.extend(renderer, &nodes);
            if self.nodes_buffer.capacity != old_capacity {
                // The buffer capacity changed, so we have a new buffer; recreate the bind group.
                self.nodes_bind_group = Self::create_nodes_bind_group(
                    &renderer.device,
                    &self.nodes_bind_group_layout,
                    self.nodes_buffer.buffer(),
                );
            }

            range
        };

        let render_model = self.models.insert(RenderModel {
            opaque_range,
            alpha_range,
            _additive_range: additive_range,
            nodes_range,
        });

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

    fn create_nodes_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model_nodes_bind_group"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        })
    }
}
