use std::ops::Range;

use bytemuck::{NoUninit, cast_slice};

use crate::engine::prelude::Renderer;

pub struct GrowingBuffer<T: NoUninit> {
    /// Label used for the buffer.
    label: String,
    /// Usages for the buffer. COPY_DST is always added.
    usage: wgpu::BufferUsages,
    /// Handle to the underlying buffer.
    buffer: wgpu::Buffer,
    /// Current amount of items in the buffer.
    count: u32,
    /// Amount of items that can be held in the buffer.
    capacity: u32,

    _phantom: std::marker::PhantomData<T>,
}

impl<T: NoUninit> GrowingBuffer<T> {
    const STRIDE: u64 = std::mem::size_of::<T>() as u64;

    pub fn new(
        renderer: &Renderer,
        capacity: u32,
        usage: wgpu::BufferUsages,
        label: impl Into<String>,
    ) -> Self {
        let label = label.into();
        let size = Self::STRIDE * capacity as u64;
        let buffer = Self::create_buffer(renderer, &label, size, usage);

        Self {
            label,
            usage,
            buffer,
            count: 0,
            capacity,
            _phantom: std::marker::PhantomData,
        }
    }

    #[inline]
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    #[inline]
    pub fn slice<S: std::ops::RangeBounds<wgpu::BufferAddress>>(
        &self,
        range: S,
    ) -> wgpu::BufferSlice {
        self.buffer.slice(range)
    }

    /// Write the given data to the start of the buffer. Returns the range where it was written.
    pub fn write(&mut self, renderer: &Renderer, data: &[T]) -> Range<u32> {
        self.ensure_size(renderer, data.len() as u32);

        renderer
            .queue
            .write_buffer(&self.buffer, 0, cast_slice(data));

        self.count = data.len() as u32;

        0..self.count
    }

    /// Write the given data to the end of the buffer. Returns the range where it was written.
    pub fn extend(&mut self, renderer: &Renderer, data: &[T]) -> Range<u32> {
        self.ensure_size(renderer, self.count + data.len() as u32);

        let start = self.count;

        let offset = Self::STRIDE * start as u64;

        renderer
            .queue
            .write_buffer(&self.buffer, offset, cast_slice(data));

        self.count += data.len() as u32;
        let end = self.count;

        start..end
    }

    fn ensure_size(&mut self, renderer: &Renderer, required_capacity: u32) {
        if required_capacity >= self.capacity {
            self.resize(renderer, required_capacity.next_power_of_two());
        }
    }

    fn resize(&mut self, renderer: &Renderer, capacity: u32) {
        self.capacity = capacity;

        let size_in_bytes = self.capacity as u64 * Self::STRIDE;

        tracing::info!(
            "Growing buffer with label \"{}\" to {} ({} bytes).",
            self.label,
            capacity,
            size_in_bytes,
        );

        let buffer = Self::create_buffer(renderer, &self.label, size_in_bytes, self.usage);

        let mut encoder = renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some(&format!("{}_grow", self.label)),
            });

        encoder.copy_buffer_to_buffer(
            &self.buffer,
            0,
            &buffer,
            0,
            self.count as u64 * Self::STRIDE,
        );

        renderer.queue.submit(std::iter::once(encoder.finish()));

        self.buffer = buffer;
    }

    fn create_buffer(
        renderer: &Renderer,
        label: &str,
        size: u64,
        usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{label}_buffer")),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC | usage,
            mapped_at_creation: false,
        })
    }
}
