use std::ops::Range;

use ahash::HashMap;
use wgpu::util::DeviceExt;

use crate::{
    engine::{mesh::IndexedMesh, renderer::Gpu, storage::Handle},
    game::{
        assets::{image::BlendMode, model::Model, models::Models},
        globals,
        render::textures::Texture,
    },
};

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
pub struct RenderVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
    pub node_index: u32,
}

/// One drawable mesh: a contiguous range in its [RenderModel]'s index buffer
/// plus the texture used to sample its pixels.
pub struct RenderMesh {
    pub index_range: Range<u32>,
    pub texture: Handle<Texture>,
}

/// GPU-side data for a single [Model]. Each model owns its own vertex/index/nodes
/// buffers and the bind group used to access the nodes during rendering.
pub struct RenderModel {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub _nodes_buffer: wgpu::Buffer,
    pub nodes_bind_group: wgpu::BindGroup,
    pub opaque_meshes: Vec<RenderMesh>,
    pub keyed_meshes: Vec<RenderMesh>,
    pub alpha_meshes: Vec<RenderMesh>,
}

pub struct RenderModels {
    pub nodes_bind_group_layout: wgpu::BindGroupLayout,
    models: HashMap<Handle<Model>, RenderModel>,
}

impl RenderModels {
    pub fn new(gpu: &Gpu) -> Self {
        let nodes_bind_group_layout =
            gpu.device
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

        Self {
            nodes_bind_group_layout,
            models: HashMap::default(),
        }
    }

    pub fn add(&mut self, asset_models: &Models, gpu: &Gpu, model_handle: Handle<Model>) {
        if self.models.contains_key(&model_handle) {
            return;
        }

        let model = asset_models
            .get(model_handle)
            .expect("Model should have been loaded by this time.");

        // Precompose each bone's local-to-model transform on the CPU so the
        // vertex shader can do a single buffer lookup instead of walking the
        // parent chain per vertex.
        let nodes: Vec<[[f32; 4]; 4]> = (0..model.skeleton.bones.len() as u32)
            .map(|i| model.skeleton.local_transform(i).to_cols_array_2d())
            .collect();

        // Build the per-model vertex and index buffers by concatenating all
        // mesh data, with each mesh's indices rebased onto where its vertices
        // land in the combined vertex buffer.
        let mut vertices: Vec<RenderVertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        let mut opaque_meshes: Vec<RenderMesh> = Vec::new();
        let mut keyed_meshes: Vec<RenderMesh> = Vec::new();
        let mut alpha_meshes: Vec<RenderMesh> = Vec::new();

        for mesh in model.meshes.iter() {
            let texture_handle = globals::textures()
                .create_from_image(mesh.image)
                .expect("Image should have been loaded by this time.");
            let texture_data = globals::textures()
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

            let vertex_offset = vertices.len() as u32;
            vertices.append(&mut indexed_mesh.vertices);

            let index_start = indices.len() as u32;
            indices.extend(indexed_mesh.indices.iter().map(|i| i + vertex_offset));
            let index_end = indices.len() as u32;

            let render_mesh = RenderMesh {
                index_range: index_start..index_end,
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

        let device = &gpu.device;

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("render_model_vertex_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("render_model_index_buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let nodes_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("render_model_nodes_buffer"),
            contents: bytemuck::cast_slice(&nodes),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let nodes_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render_model_nodes_bind_group"),
            layout: &self.nodes_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: nodes_buffer.as_entire_binding(),
            }],
        });

        self.models.insert(
            model_handle,
            RenderModel {
                vertex_buffer,
                index_buffer,
                _nodes_buffer: nodes_buffer,
                nodes_bind_group,
                opaque_meshes,
                keyed_meshes,
                alpha_meshes,
            },
        );
    }

    #[inline]
    pub fn get(&self, handle: Handle<Model>) -> Option<&RenderModel> {
        self.models.get(&handle)
    }

    #[inline]
    pub fn contains(&self, handle: Handle<Model>) -> bool {
        self.models.contains_key(&handle)
    }
}
