use glam::UVec2;

use crate::engine::renderer::Renderer;

pub struct GeometryBuffers {
    pub colors_buffer: wgpu::TextureView,
    pub positions_buffer: wgpu::TextureView,
    pub normals_buffer: wgpu::TextureView,
    pub ids_buffer: wgpu::TextureView,

    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl GeometryBuffers {
    pub fn new(renderer: &Renderer) -> Self {
        let size = UVec2::new(
            renderer.surface_config.width,
            renderer.surface_config.height,
        );

        let colors_buffer = Self::create_buffer(
            renderer,
            "g_buffer_colors",
            size,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );

        let positions_buffer = Self::create_buffer(
            renderer,
            "g_buffer_positions",
            size,
            wgpu::TextureFormat::Rgba8Unorm,
        );

        let normals_buffer = Self::create_buffer(
            renderer,
            "g_buffer_positions",
            size,
            wgpu::TextureFormat::Rgba8Unorm,
        );

        let ids_buffer = Self::create_buffer(
            renderer,
            "g_buffer_positions",
            size,
            wgpu::TextureFormat::R8Uint,
        );

        let bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("g_buffer_bind_group_layout"),
                    entries: &[
                        // t_colors
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        // t_positions
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        // t_normals
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        // t_entity_ids
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Uint,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                    ],
                });

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("g_buffer_bind_group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&colors_buffer),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&positions_buffer),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&normals_buffer),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&ids_buffer),
                    },
                ],
            });

        Self {
            colors_buffer,
            positions_buffer,
            normals_buffer,
            ids_buffer,

            bind_group_layout,
            bind_group,
        }
    }

    pub fn targets() -> &'static [Option<wgpu::ColorTargetState>] {
        &[
            Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::R8Uint,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
        ]
    }

    fn create_buffer(
        renderer: &Renderer,
        label: &str,
        size: UVec2,
        format: wgpu::TextureFormat,
    ) -> wgpu::TextureView {
        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }
}
