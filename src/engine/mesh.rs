use wgpu::util::DeviceExt;

use crate::engine::renderer::renderer;

#[derive(Clone)]
pub struct IndexedMesh<V> {
    pub vertices: Vec<V>,
    pub indices: Vec<u32>,
}

impl<V> std::fmt::Debug for IndexedMesh<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexedMesh")
            .field("vertices", &self.vertices.len())
            .field("indices", &self.indices.len())
            .finish()
    }
}

impl<V: Copy> IndexedMesh<V> {
    pub fn _new(vertices: Vec<V>, indices: Vec<u32>) -> Self {
        Self { vertices, indices }
    }

    pub fn _is_empty(&self) -> bool {
        self.vertices.is_empty() || self.indices.is_empty()
    }

    pub fn extend(&mut self, mesh: Self) -> std::ops::Range<u32> {
        let vertex_offset = self.vertices.len() as u32;

        self.vertices.extend(mesh.vertices);

        let first_index = self.indices.len() as u32;

        self.indices
            .extend(mesh.indices.iter().map(|i| i + vertex_offset));

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
    pub fn to_gpu(&self, device: &wgpu::Device) -> GpuIndexedMesh {
        debug_assert!(!self.vertices.is_empty(), "Uploading empty vertex buffer.");
        debug_assert!(!self.indices.is_empty(), "Uploading empty index buffer.");

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh_vertex_buffer"),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
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
