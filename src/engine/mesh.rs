use glam::{Vec2, Vec3};

use super::{
    assets::Asset,
    renderer::{BufferLayout, Renderer},
};

#[derive(Clone, Copy, Debug, bytemuck::NoUninit)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub tex_coord: Vec2,
}

impl BufferLayout for Vertex {
    fn vertex_buffers() -> &'static [wgpu::VertexBufferLayout<'static>] {
        use wgpu::vertex_attr_array;

        const VERTEX_ATTR_ARRAY: &[wgpu::VertexAttribute] = &vertex_attr_array!(
            0 => Float32x3, // position
            1 => Float32x3, // normal
            2 => Float32x2, // tex_coord
        );

        &[wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: VERTEX_ATTR_ARRAY,
        }]
    }
}

#[derive(Default)]
pub struct Mesh<V: BufferLayout> {
    pub vertices: Vec<V>,
    pub indices: Vec<u32>,
}

pub struct GpuMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

impl Asset for GpuMesh {}

impl std::fmt::Debug for GpuMesh {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GpuMesh")
            .field("vertex_buffer", &self.vertex_buffer)
            .field("index_buffer", &self.index_buffer)
            .field("index_count", &self.index_count)
            .finish()
    }
}

impl<V: BufferLayout + bytemuck::NoUninit> Mesh<V> {
    pub fn to_gpu(&self, renderer: &Renderer) -> GpuMesh {
        debug_assert!(!self.vertices.is_empty(), "Uploading empty vertex buffer.");
        debug_assert!(!self.indices.is_empty(), "Uploading empty index buffer.");

        let vertex_buffer = renderer.create_vertex_buffer("mesh_vertex_buffer", &self.vertices);
        let index_buffer = renderer.create_index_buffer("mesh_index_buffer", &self.indices);

        GpuMesh {
            vertex_buffer,
            index_buffer,
            index_count: self.indices.len() as u32,
        }
    }
}

pub trait RenderPassMeshExt {
    fn draw_mesh(&mut self, mesh: &GpuMesh, instances: std::ops::Range<u32>);
}

impl<'encoder> RenderPassMeshExt for wgpu::RenderPass<'encoder> {
    fn draw_mesh(&mut self, mesh: &GpuMesh, instances: std::ops::Range<u32>) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.draw_indexed(0..mesh.index_count, 0, instances);
    }
}
