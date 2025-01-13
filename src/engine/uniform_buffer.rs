use std::cell::RefCell;

use wgpu::util::DeviceExt;

use crate::{RenderQueue, Renderer};

pub struct UniformBuffer<B: bytemuck::NoUninit + Default> {
    buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,

    data: RefCell<B>,

    _phantom: std::marker::PhantomData<B>,
}

impl<B: bytemuck::NoUninit + Default> UniformBuffer<B> {
    pub fn new(renderer: &Renderer, label: &str, visibility: wgpu::ShaderStages) -> Self {
        // SAFETY: Unchecked here, because the size of the struct *MUST* be more than 0, otherwise
        //         what is the point.
        let buffer_size =
            unsafe { std::num::NonZeroU64::new_unchecked(std::mem::size_of::<B>() as u64) };

        let data = RefCell::new(B::default());

        let buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(&[*data.borrow()]),
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

            data,

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
