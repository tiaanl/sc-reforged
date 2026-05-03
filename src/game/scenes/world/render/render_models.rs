use std::ops::Range;

use crate::{
    engine::{
        assets::AssetError,
        growing_buffer::GrowingBuffer,
        mesh::IndexedMesh,
        renderer::RenderContext,
        storage::{Handle, Storage},
    },
    game::{
        assets::{image::BlendMode, model::Model, models::Models},
        render::textures::{Texture, Textures},
    },
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
}

/// One drawable mesh: a contiguous range in the global index buffer plus the
/// texture used to sample its pixels.
pub struct RenderMesh {
    pub index_range: Range<u32>,
    pub texture: Handle<Texture>,
}

/// GPU-side data for a single [Model], with meshes split per blend mode so the
/// queue path can pick them up in the matching render pass.
pub struct RenderModel {
    pub opaque_meshes: Vec<RenderMesh>,
    pub keyed_meshes: Vec<RenderMesh>,
    pub alpha_meshes: Vec<RenderMesh>,
    /// Range of [RenderNode]s for this model in the shared nodes buffer.
    pub nodes_range: Range<u32>,
}

pub struct RenderModels {
    /// Buffer containing all model vertices.
    vertices_buffer: GrowingBuffer<RenderVertex>,
    /// Buffer containing all model indices.
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

    pub fn new(context: &RenderContext) -> Self {
        let vertices_buffer = GrowingBuffer::new(
            context,
            Self::INITIAL_VERTEX_COUNT,
            wgpu::BufferUsages::VERTEX,
            "render_models_vertices",
        );

        let indices_buffer = GrowingBuffer::new(
            context,
            Self::INITIAL_INDEX_COUNT,
            wgpu::BufferUsages::INDEX,
            "render_models_indices",
        );

        let nodes_buffer = GrowingBuffer::new(
            context,
            Self::INITIAL_NODES_COUNT,
            wgpu::BufferUsages::STORAGE,
            "render_models_nodes",
        );

        let nodes_bind_group_layout =
            context
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
            &context.device,
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
        textures: &Textures,
        models: &Models,
        context: &RenderContext,
        model_handle: Handle<Model>,
    ) -> Result<Handle<RenderModel>, AssetError> {
        let model = models
            .get(model_handle)
            .expect("Model should have been loaded by this time.");

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

        let mut opaque_meshes: Vec<RenderMesh> = Vec::new();
        let mut keyed_meshes: Vec<RenderMesh> = Vec::new();
        let mut alpha_meshes: Vec<RenderMesh> = Vec::new();

        for mesh in model.meshes.iter() {
            let texture_handle = textures
                .create_from_image(mesh.image)
                .expect("Image should have been loaded by this time.");
            let texture_data = textures
                .get(texture_handle)
                .expect("Texture should exist immediately after creation.");

            let mut indexed_mesh: IndexedMesh<RenderVertex> = IndexedMesh {
                vertices: mesh
                    .mesh
                    .vertices
                    .iter()
                    .map(|v| RenderVertex {
                        position: v.position.to_array(),
                        normal: v.normal.to_array(),
                        tex_coord: v.tex_coord.to_array(),
                        node_index: v.node_index,
                    })
                    .collect(),
                indices: mesh.mesh.indices.clone(),
            };

            let (vertices_range, _) = self.vertices_buffer.extend(context, &indexed_mesh.vertices);
            // Adjust the indices to reference the global vertex range.
            indexed_mesh
                .indices
                .iter_mut()
                .for_each(|i| *i += vertices_range.start);

            let (index_range, _) = self.indices_buffer.extend(context, &indexed_mesh.indices);

            let render_mesh = RenderMesh {
                index_range,
                texture: texture_handle,
            };

            match texture_data.blend_mode {
                BlendMode::Opaque => opaque_meshes.push(render_mesh),
                BlendMode::ColorKeyed => keyed_meshes.push(render_mesh),
                BlendMode::Alpha => alpha_meshes.push(render_mesh),
                BlendMode::_Additive => {
                    // Additive blending is not currently rendered.
                }
            }
        }

        let nodes_range = {
            let old_capacity = self.nodes_buffer.capacity;
            let (range, _) = self.nodes_buffer.extend(context, &nodes);
            if self.nodes_buffer.capacity != old_capacity {
                self.nodes_bind_group = Self::create_nodes_bind_group(
                    &context.device,
                    &self.nodes_bind_group_layout,
                    self.nodes_buffer.buffer(),
                );
            }
            range
        };

        let render_model = self.models.insert(RenderModel {
            opaque_meshes,
            keyed_meshes,
            alpha_meshes,
            nodes_range,
        });

        Ok(render_model)
    }

    #[inline]
    pub fn get(&self, handle: Handle<RenderModel>) -> Option<&RenderModel> {
        self.models.get(handle)
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
