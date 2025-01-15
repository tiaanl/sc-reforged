use std::cell::RefCell;

use bytemuck::NoUninit;
use wgpu::util::DeviceExt;

use crate::Renderer;

pub struct UniformBuffer<B: NoUninit + Default> {
    buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,

    data: RefCell<B>,

    _phantom: std::marker::PhantomData<B>,
}

impl<B: NoUninit + Default> UniformBuffer<B> {
    const BUFFER_SIZE: usize = std::mem::size_of::<B>();

    #[inline]
    pub fn new(renderer: &Renderer, label: &str, visibility: wgpu::ShaderStages) -> Self {
        Self::with_data(renderer, label, visibility, B::default())
    }

    pub fn with_data(
        renderer: &Renderer,
        label: &str,
        visibility: wgpu::ShaderStages,
        data: B,
    ) -> Self {
        // SAFETY: Unchecked here, because the size of the struct *MUST* be more than 0, otherwise
        //         what is the point.
        let buffer_size = unsafe { std::num::NonZeroU64::new_unchecked(Self::BUFFER_SIZE as u64) };

        let buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(&[data]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some(label),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: Some(buffer_size),
                        },
                        count: None,
                    }],
                });

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });

        Self {
            buffer,
            bind_group_layout,
            bind_group,

            data: RefCell::new(data),

            _phantom: std::marker::PhantomData,
        }
    }

    pub fn upload(&self, queue: &wgpu::Queue, mut f: impl FnMut(&mut B)) {
        f(&mut *self.data.borrow_mut());
        queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::cast_slice(&[*self.data.borrow()]),
        );
    }
}

pub struct StorageBuffer<B: NoUninit + Default> {
    buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,

    data: RefCell<Vec<B>>,

    _phantom: std::marker::PhantomData<B>,
}

impl<B: NoUninit + Default> StorageBuffer<B> {
    const BUFFER_SIZE: usize = std::mem::size_of::<B>();

    pub fn new(
        renderer: &Renderer,
        label: &str,
        visibility: wgpu::ShaderStages,
        read_only: bool,
        data: Vec<B>,
    ) -> Self {
        // SAFETY: Unchecked here, because the size of the struct *MUST* be more than 0, otherwise
        //         what is the point.
        let buffer_size = unsafe { std::num::NonZeroU64::new_unchecked(Self::BUFFER_SIZE as u64) };

        let buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(&data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some(label),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only },
                            has_dynamic_offset: false,
                            min_binding_size: Some(buffer_size), // At least 1.
                        },
                        // count: Some(unsafe { std::num::NonZeroU32::new_unchecked(capacity) }),
                        count: None,
                    }],
                });

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(buffer.as_entire_buffer_binding()),
                }],
            });

        Self {
            buffer,
            bind_group_layout,
            bind_group,
            data: RefCell::new(data),
            _phantom: std::marker::PhantomData,
        }
    }
}
