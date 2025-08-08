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
    game::{image::BlendMode, model, models::models},
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

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
pub struct RenderNode {
    pub transform: [f32; 16],
    pub parent_node_index: NodeIndex,
    pub _padding: [u32; 3],
}

struct RenderMesh {
    indices: Range<u32>,
    texture_handle: Handle<RenderTexture>,
}

pub struct RenderNodeBuffer {
    pub _buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl RenderNodeBuffer {
    pub fn from_nodes(
        nodes_bind_group_layout: &wgpu::BindGroupLayout,
        nodes: &[RenderNode],
    ) -> Self {
        let buffer = renderer()
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("model_renderer_animated_node_buffer"),
                contents: bytemuck::cast_slice(nodes),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let bind_group = renderer()
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("model_renderer_animated_node_buffer_bind_group"),
                layout: nodes_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &buffer,
                        offset: 0,
                        size: None,
                    }),
                }],
            });

        Self {
            _buffer: buffer,
            bind_group,
        }
    }

    pub fn update_from_nodes(&self, nodes: &[RenderNode]) {
        renderer()
            .queue
            .write_buffer(&self._buffer, 0, bytemuck::cast_slice(nodes));
    }
}

pub struct RenderModel {
    /// Contains the vertices for the entire model.
    vertex_buffer: wgpu::Buffer,
    /// Contains the indices for the entire model.
    index_buffer: wgpu::Buffer,
    /// For binding the nodes to the shader.
    nodes: RenderNodeBuffer,
    /// For binding the animated nodes to the shader.
    pub animated_nodes: Option<RenderNodeBuffer>,

    /// All the meshes (sets of indices) that the model consists of.
    meshes: Vec<RenderMesh>,

    pub scale: Vec3,
}

impl RenderModel {
    pub fn render(
        &self,
        render_pass: &mut wgpu::RenderPass,
        textures: &RenderTextures,
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
            if let Some(ref animated_nodes) = self.animated_nodes {
                render_pass.set_bind_group(3, &animated_nodes.bind_group, &[]);
            } else {
                render_pass.set_bind_group(3, &self.nodes.bind_group, &[]);
            }
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(mesh.indices.clone(), 0, 0..1);
        }
    }
}

pub struct RenderModels {
    models: Storage<RenderModel>,

    pub nodes_bind_group_layout: wgpu::BindGroupLayout,
}

impl RenderModels {
    pub fn new() -> Self {
        let nodes_bind_group_layout =
            renderer()
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("model_renderer_nodes_bind_group_layout"),
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
            models: Storage::default(),

            nodes_bind_group_layout,
        }
    }

    pub fn add_model(
        &mut self,
        textures: &mut RenderTextures,
        model_handle: Handle<model::Model>,
    ) -> Result<Handle<RenderModel>, AssetError> {
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

        let nodes: Vec<RenderNode> = model
            .nodes
            .iter()
            .enumerate()
            .map(|(node_index, node)| {
                let transform = model.local_transform(node_index as u32);
                RenderNode {
                    transform: transform.to_cols_array(),
                    parent_node_index: node.parent,
                    _padding: [0; 3],
                }
            })
            .collect();

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

        let nodes = RenderNodeBuffer::from_nodes(&self.nodes_bind_group_layout, &nodes);

        Ok(self.models.insert(RenderModel {
            vertex_buffer,
            index_buffer,
            nodes,
            animated_nodes: None,
            meshes,

            scale: model.scale,
        }))
    }

    pub fn update_animation_nodes(&mut self, handle: Handle<RenderModel>, nodes: &[RenderNode]) {
        let Some(model) = self.models.get_mut(handle) else {
            tracing::warn!("Trying to update nodes for a model that does not exist!");
            return;
        };

        if let Some(ref mut animated_nodes) = model.animated_nodes {
            animated_nodes.update_from_nodes(nodes);
        } else {
            model.animated_nodes = Some(RenderNodeBuffer::from_nodes(
                &self.nodes_bind_group_layout,
                nodes,
            ));
        }
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
