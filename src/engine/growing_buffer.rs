use std::ops::Range;

use bytemuck::{NoUninit, cast_slice};

use crate::engine::renderer::renderer;

pub struct GrowingBuffer<T: NoUninit> {
    /// Label used for the buffer.
    label: String,
    /// Usages for the buffer. COPY_DST is always added.
    usage: wgpu::BufferUsages,
    /// Handle to the underlying buffer.
    buffer: wgpu::Buffer,
    /// Amount of items that can be held in the buffer.
    capacity: u32,
    /// Points to the first open item slot.
    cursor: u32,

    _phantom: std::marker::PhantomData<T>,
}

impl<T: NoUninit> GrowingBuffer<T> {
    const STRIDE: u64 = std::mem::size_of::<T>() as u64;

    pub fn new(capacity: u32, usage: wgpu::BufferUsages, label: impl Into<String>) -> Self {
        let label = label.into();
        let size = Self::STRIDE * capacity as u64;
        let buffer = Self::create_buffer(&label, size, usage);

        Self {
            label,
            usage,
            buffer,
            capacity,
            cursor: 0,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn buffer_slice(&self) -> wgpu::BufferSlice<'_> {
        self.buffer.slice(..)
    }

    /// Write the given data into the buffer at the cursor location. Returns the range where the
    /// data was written to.
    pub fn push(&mut self, data: &[T]) -> Range<u32> {
        let required_item_count = self.cursor + data.len() as u32;
        if required_item_count > self.capacity {
            self.resize(required_item_count.next_power_of_two());
        }

        let start = self.cursor;

        let offset = Self::STRIDE * self.cursor as u64;
        renderer()
            .queue
            .write_buffer(&self.buffer, offset, cast_slice(data));

        self.cursor += data.len() as u32;
        let end = self.cursor;

        start..end
    }

    fn resize(&mut self, capacity: u32) {
        self.capacity = capacity;

        let size_in_bytes = self.capacity as u64 * Self::STRIDE;

        tracing::info!(
            "Growing buffer with label \"{}\" to {} ({} bytes).",
            self.label,
            capacity,
            size_in_bytes,
        );

        let buffer = Self::create_buffer(&self.label, size_in_bytes, self.usage);

        let mut encoder =
            renderer()
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some(&format!("{}_grow", self.label)),
                });

        encoder.copy_buffer_to_buffer(
            &self.buffer,
            0,
            &buffer,
            0,
            self.cursor as u64 * Self::STRIDE,
        );

        renderer().queue.submit(std::iter::once(encoder.finish()));

        self.buffer = buffer;
    }

    fn create_buffer(label: &str, size: u64, usage: wgpu::BufferUsages) -> wgpu::Buffer {
        renderer().device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{label}_buffer")),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC | usage,
            mapped_at_creation: false,
        })
    }
}
