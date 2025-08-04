use wgpu::util::DeviceExt;

use crate::engine::renderer::renderer;

#[derive(Debug)]
pub struct IndexedMesh<V> {
    pub vertices: Vec<V>,
    pub indices: Vec<u32>,
}

impl<V: Clone> IndexedMesh<V> {
    pub fn _extend(&mut self, mesh: &Self) -> std::ops::Range<u32> {
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
