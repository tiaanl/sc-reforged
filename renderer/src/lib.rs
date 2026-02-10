//! A wrapper around `wgpu` primitives to render graphics to a surface.

mod bind_groups;
mod buffers;

pub use bind_groups::*;
pub use buffers::*;

use generational_arena::Arena;

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,

    buffers: Arena<BufferEntry>,
}

impl Renderer {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        Self {
            device,
            queue,
            buffers: Arena::default(),
        }
    }

    pub fn create_buffer(&mut self, descriptor: BufferDescriptor) -> BufferId {
        BufferId(self.buffers.insert(BufferEntry { descriptor }))
    }
}

struct BufferEntry {
    pub descriptor: BufferDescriptor,
}
