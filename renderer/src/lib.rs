//! A wrapper around `wgpu` primitives to render graphics to a surface.

mod buffers;

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
        BufferId(self.buffers.insert(BufferEntry {
            descriptor,
            buffer: None,
        }))
    }
}

struct BufferEntry {
    pub descriptor: BufferDescriptor,
    pub buffer: Option<wgpu::Buffer>,
}
