use glam::Vec3;

use crate::game::config::Fog;

use super::Renderer;

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
struct RawFog {
    color: Vec3,    // 12
    _padding: f32,  // 4
    start: f32,     // 4
    end: f32,       // 4
    density: f32,   // 4
    _padding2: f32, // 4
}

pub struct GpuFog {
    buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl GpuFog {
    pub fn new(renderer: &Renderer) -> Self {
        let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fog_buffer"),
            size: std::mem::size_of::<RawFog>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("fog_bind_group_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("fog_bind_group"),
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
        }
    }

    pub fn upload(&self, queue: &wgpu::Queue, fog: &Fog, density: f32) {
        let raw_fog = RawFog {
            color: fog.color,
            _padding: 0.0,
            start: fog.start,
            end: fog.end,
            density,
            _padding2: 0.0,
        };
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[raw_fog]));
    }
}
