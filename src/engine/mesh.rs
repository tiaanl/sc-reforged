use glam::{Vec2, Vec3};
use wgpu::util::DeviceExt;

use super::renderer::Renderer;

#[derive(Clone, Copy, Debug, bytemuck::NoUninit)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub tex_coord: Vec2,
}

impl Vertex {
    pub fn from_position(position: Vec3) -> Self {
        Self {
            position,
            normal: Vec3::ZERO,
            tex_coord: Vec2::ZERO,
        }
    }

    pub fn new(position: Vec3, normal: Vec3, tex_coord: Vec2) -> Self {
        Self {
            position,
            normal,
            tex_coord,
        }
    }
}

#[derive(Default)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

pub struct GpuMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

impl Mesh {
    pub fn to_gpu(&self, renderer: &Renderer) -> GpuMesh {
        debug_assert!(!self.vertices.is_empty(), "Uploading empty vertex buffer.");
        debug_assert!(!self.indices.is_empty(), "Uploading empty index buffer.");

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

        GpuMesh {
            vertex_buffer,
            index_buffer,
            index_count: self.indices.len() as u32,
        }
    }
}

pub trait RenderPassMeshExt {
    fn draw_mesh(&mut self, mesh: &GpuMesh);
}

impl<'encoder> RenderPassMeshExt for wgpu::RenderPass<'encoder> {
    fn draw_mesh(&mut self, mesh: &GpuMesh) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.draw_indexed(0..mesh.index_count, 0, 0..1);
    }
}
