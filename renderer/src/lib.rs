//! A wrapper around `wgpu` primitives to render graphics to a surface.

mod bind_groups;
mod buffers;
mod pipelines;
mod shaders;
mod vertex_layouts;

pub use bind_groups::*;
pub use buffers::*;
pub use pipelines::*;
pub use shaders::*;
pub use vertex_layouts::*;

use generational_arena::Arena;
use std::borrow::Cow;

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,

    buffers: Arena<BufferEntry>,
    shaders: Arena<ShaderEntry>,
    color_targets: Arena<ColorTargetEntry>,
    render_pipelines: Arena<RenderPipelineEntry>,
}

impl Renderer {
    /// Creates a new renderer wrapper around a `wgpu` device and queue.
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        Self {
            device,
            queue,
            buffers: Arena::default(),
            shaders: Arena::default(),
            color_targets: Arena::default(),
            render_pipelines: Arena::default(),
        }
    }

    /// Creates a tracked buffer entry and returns its handle.
    pub fn create_buffer(&mut self, descriptor: BufferDescriptor) -> BufferId {
        BufferId(self.buffers.insert(BufferEntry { descriptor }))
    }

    /// Creates a shader module from WGSL source and returns its handle.
    pub fn create_shader(&mut self, source: &str) -> ShaderId {
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(Cow::Owned(source.to_string())),
            });

        ShaderId(self.shaders.insert(ShaderEntry { module }))
    }

    /// Stores a minimal color target descriptor and returns its handle.
    pub fn create_color_target(&mut self, descriptor: ColorTargetDescriptor) -> ColorTargetId {
        ColorTargetId(self.color_targets.insert(ColorTargetEntry { descriptor }))
    }

    /// Stores a render pipeline descriptor and returns its handle.
    ///
    /// The input vertex layout is sourced from `V`, which should usually derive
    /// `renderer_macros::AsVertexLayout`.
    pub fn create_render_pipeline<V: AsVertexLayout>(
        &mut self,
        descriptor: RenderPipelineDescriptor,
    ) -> Option<RenderPipelineId> {
        if descriptor.output_targets.is_empty() {
            tracing::warn!("Skipping render pipeline registration: no output targets were set.");
            return None;
        }

        if self.shaders.get(descriptor.vertex_module.0).is_none() {
            tracing::warn!(
                "Skipping render pipeline registration: invalid vertex shader id ({:?}).",
                descriptor.vertex_module.0
            );
            return None;
        }

        if self.shaders.get(descriptor.fragment_module.0).is_none() {
            tracing::warn!(
                "Skipping render pipeline registration: invalid fragment shader id ({:?}).",
                descriptor.fragment_module.0
            );
            return None;
        }

        if descriptor
            .output_targets
            .iter()
            .any(|id| self.color_targets.get(id.0).is_none())
        {
            tracing::warn!(
                "Skipping render pipeline registration: one or more color target ids are invalid."
            );
            return None;
        }

        let vertex_layout = V::vertex_buffer_layout();

        Some(RenderPipelineId(self.render_pipelines.insert(
            RenderPipelineEntry {
                descriptor,
                vertex_layout,
            },
        )))
    }

    /// Returns a stored render pipeline descriptor by handle.
    pub fn render_pipeline_descriptor(
        &self,
        id: RenderPipelineId,
    ) -> Option<&RenderPipelineDescriptor> {
        self.render_pipelines
            .get(id.0)
            .map(|entry| &entry.descriptor)
    }

    /// Returns the stored vertex layout for a render pipeline descriptor handle.
    pub fn render_pipeline_vertex_layout(
        &self,
        id: RenderPipelineId,
    ) -> Option<&VertexBufferLayout> {
        self.render_pipelines
            .get(id.0)
            .map(|entry| &entry.vertex_layout)
    }

    /// Returns a stored shader module by handle.
    pub fn shader(&self, id: ShaderId) -> Option<&wgpu::ShaderModule> {
        self.shaders.get(id.0).map(|entry| &entry.module)
    }

    /// Returns a stored color target descriptor by handle.
    pub fn color_target_descriptor(&self, id: ColorTargetId) -> Option<&ColorTargetDescriptor> {
        self.color_targets.get(id.0).map(|entry| &entry.descriptor)
    }
}

struct BufferEntry {
    pub descriptor: BufferDescriptor,
}

struct ShaderEntry {
    module: wgpu::ShaderModule,
}
