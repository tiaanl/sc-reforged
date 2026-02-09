#![allow(unused)]

use std::hash::{Hash, Hasher};

use ahash::HashMap;

use crate::{engine::shader_cache::ShaderSource, game::scenes::world::render::RenderLayouts};

#[derive(Hash, Eq, PartialEq)]
pub struct BindGroupLayoutDescriptor {
    pub label: String,
    pub entries: Vec<wgpu::BindGroupLayoutEntry>,
}

#[derive(Hash, Eq, PartialEq)]
pub struct VertexBufferLayout {
    pub array_stride: wgpu::BufferAddress,
    pub step_mode: wgpu::VertexStepMode,
    pub attributes: Vec<wgpu::VertexAttribute>,
}

#[derive(Hash, Eq, PartialEq)]
pub struct VertexState {
    pub shader: ShaderSource,
    pub entry_point: String,
    pub buffers: Vec<VertexBufferLayout>,
}

#[derive(Hash, Eq, PartialEq)]
pub struct PrimitiveState {
    pub topology: wgpu::PrimitiveTopology,
    pub cull_mode: wgpu::Face,
    pub polygon_mode: wgpu::PolygonMode,
}

impl Default for PrimitiveState {
    fn default() -> Self {
        Self {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: wgpu::Face::Back,
            polygon_mode: wgpu::PolygonMode::Fill,
        }
    }
}

#[derive(Hash, Eq, PartialEq)]
pub struct DepthState {
    pub format: wgpu::TextureFormat,
    pub enabled: bool,
    pub compare: wgpu::CompareFunction,
}

#[derive(Hash, Eq, PartialEq)]
pub enum Blend {
    Opaque,
}

#[derive(Hash, Eq, PartialEq)]
pub struct ColorTargetState {
    pub format: wgpu::TextureFormat,
    pub blend: Blend,
}

#[derive(Hash, Eq, PartialEq)]
pub struct FragmentState {
    pub shader: ShaderSource,
    pub entry_point: String,
    pub targets: Vec<Option<wgpu::ColorTargetState>>,
}

#[derive(Hash, Eq, PartialEq)]
pub struct RenderPipelineDescriptor {
    pub label: String,
    pub layout: Vec<BindGroupLayoutDescriptor>,
    pub vertex: VertexState,
    pub primitive: PrimitiveState,
    pub depth: Option<DepthState>,
    pub fragment: Option<FragmentState>,
}

pub struct PipelineCache {
    pipelines: HashMap<u64, wgpu::RenderPipeline>,
}

impl PipelineCache {
    pub fn get_or_create(
        &mut self,
        device: &wgpu::Device,
        layouts: &mut RenderLayouts,
        descriptor: &RenderPipelineDescriptor,
    ) -> Option<&wgpu::RenderPipeline> {
        let mut hasher = std::hash::DefaultHasher::new();
        descriptor.hash(&mut hasher);
        let hash = hasher.finish();

        todo!()
    }
}
