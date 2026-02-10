//! A wrapper around `wgpu` primitives to render graphics to a surface.

mod bind_groups;
mod buffers;
mod shaders;
mod vertex_layouts;

pub use bind_groups::*;
pub use buffers::*;
pub use shaders::*;
pub use vertex_layouts::*;

use generational_arena::Arena;
use std::borrow::Cow;

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,

    buffers: Arena<BufferEntry>,
    shaders: Arena<ShaderEntry>,
}

impl Renderer {
    /// Creates a new renderer wrapper around a `wgpu` device and queue.
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        Self {
            device,
            queue,
            buffers: Arena::default(),
            shaders: Arena::default(),
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
}

struct BufferEntry {
    pub descriptor: BufferDescriptor,
}

struct ShaderEntry {
    #[allow(dead_code)]
    module: wgpu::ShaderModule,
}
