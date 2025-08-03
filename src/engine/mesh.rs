use glam::{Vec2, Vec3};
use wgpu::util::DeviceExt;

use crate::engine::prelude::renderer;

use super::renderer::BufferLayout;

#[derive(Clone, Copy, Debug, bytemuck::NoUninit)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub tex_coord: Vec2,
}

impl BufferLayout for Vertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        const VERTEX_ATTR_ARRAY: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array!(
            0 => Float32x3, // position
            1 => Float32x3, // normal
            2 => Float32x2, // tex_coord
        );

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: VERTEX_ATTR_ARRAY,
        }
    }
}

#[derive(Debug)]
pub struct IndexedMesh<V> {
    pub vertices: Vec<V>,
    pub indices: Vec<u32>,
}

impl<V: Clone> IndexedMesh<V> {
    pub fn extend(&mut self, mesh: &Self) -> std::ops::Range<u32> {
        let vertex_offset = self.vertices.len() as u32;

        self.vertices.reserve(mesh.vertices.len());
        mesh.vertices
            .iter()
            .for_each(|v| self.vertices.push(v.clone()));

        let first_index = self.indices.len() as u32;

        self.indices.reserve(mesh.indices.len());
        mesh.indices
            .iter()
            .for_each(|i| self.indices.push(i + vertex_offset));

        let last_index = self.indices.len() as u32;

        first_index..last_index
    }
}

impl<V> Default for IndexedMesh<V> {
    fn default() -> Self {
        Self {
            vertices: Vec::default(),
            indices: Vec::default(),
        }
    }
}

pub struct GpuIndexedMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

impl std::fmt::Debug for GpuIndexedMesh {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GpuMesh")
            .field("vertex_buffer", &self.vertex_buffer)
            .field("index_buffer", &self.index_buffer)
            .field("index_count", &self.index_count)
            .finish()
    }
}

impl<V: bytemuck::NoUninit> IndexedMesh<V> {
    pub fn to_gpu(&self) -> GpuIndexedMesh {
        debug_assert!(!self.vertices.is_empty(), "Uploading empty vertex buffer.");
        debug_assert!(!self.indices.is_empty(), "Uploading empty index buffer.");

        let renderer = renderer();

        let vertex_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("mesh_vertex_buffer"),
                contents: bytemuck::cast_slice(&self.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("mesh_index_buffer"),
                contents: bytemuck::cast_slice(&self.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        GpuIndexedMesh {
            vertex_buffer,
            index_buffer,
            index_count: self.indices.len() as u32,
        }
    }
}

/// Mesh data for multiple meshes where each mesh is tracked by the range of indices.
#[allow(unused)]
pub struct MegaIndexedMesh<V: BufferLayout> {
    vertices: Vec<V>,
    indices: Vec<u32>,
}

impl<V: BufferLayout> Default for MegaIndexedMesh<V> {
    fn default() -> Self {
        Self {
            vertices: vec![],
            indices: vec![],
        }
    }
}

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
#[allow(unused)]
pub struct MeshSlice {
    pub start: u32,
    pub end: u32,
}

#[allow(unused)]
impl<V: BufferLayout> MegaIndexedMesh<V> {
    pub fn push_mesh(&mut self, mesh: IndexedMesh<V>) -> MeshSlice {
        let start_vertex = self.vertices.len();
        let start_index = self.indices.len();

        self.vertices.extend(mesh.vertices);

        self.indices.reserve(mesh.indices.len());
        mesh.indices
            .iter()
            .for_each(|i| self.indices.push(*i + start_vertex as u32));

        MeshSlice {
            start: start_index as u32,
            end: self.indices.len() as u32,
        }
    }
}
