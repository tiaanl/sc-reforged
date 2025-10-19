#![allow(unused)]

use glam::{UVec2, Vec3};

pub struct RenderTarget {
    pub view: wgpu::TextureView,
}

impl RenderTarget {
    pub fn new(
        device: &wgpu::Device,
        label: &str,
        size: UVec2,
        format: wgpu::TextureFormat,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: size.x.max(1),
            height: size.y.max(1),
            depth_or_array_layers: 1,
        };

        let full_label = format!("render_target_texture_{label}");

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&full_label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self { view }
    }
}

/// These [Inner] parts of a [GeometryBuffer] is recreated when the size of other parameter changes.
struct Inner {
    pub depth: RenderTarget,
    pub color: RenderTarget,
    pub oit_accumulation: RenderTarget,
    pub oit_revealage: RenderTarget,

    pub bind_group: wgpu::BindGroup,
}

impl Inner {
    fn new(device: &wgpu::Device, bind_group_layout: &wgpu::BindGroupLayout, size: UVec2) -> Self {
        tracing::info!("Creating geometry buffers ({}x{})", size.x, size.y);

        let depth = RenderTarget::new(device, "depth", size, GeometryBuffer::DEPTH_FORMAT);
        let color = RenderTarget::new(device, "color", size, GeometryBuffer::COLOR_FORMAT);
        let oit_accumulation = RenderTarget::new(
            device,
            "color",
            size,
            GeometryBuffer::OIT_ACCUMULATION_FORMAT,
        );
        let oit_revealage =
            RenderTarget::new(device, "color", size, GeometryBuffer::OIT_REVEALAGE_FORMAT);

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("g_buffer_bind_group"),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&color.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&oit_accumulation.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&oit_revealage.view),
                },
            ],
        });

        Self {
            depth,
            color,
            oit_accumulation,
            oit_revealage,

            bind_group,
        }
    }
}

pub struct GeometryBuffer {
    pub bind_group_layout: wgpu::BindGroupLayout,

    /// The current size of the buffers.
    pub size: UVec2,

    inner: Inner,
}

impl GeometryBuffer {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
    pub const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
    pub const OIT_ACCUMULATION_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
    pub const OIT_REVEALAGE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;

    pub fn new(device: &wgpu::Device, size: UVec2) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("g_buffer_bind_group_layout"),
            entries: &[
                // color
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
                // oit_accumulation
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
                // oit_revealage
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
            ],
        });

        let inner = Inner::new(device, &bind_group_layout, size);

        Self {
            bind_group_layout,
            size,
            inner,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: UVec2) {
        self.inner = Inner::new(device, &self.bind_group_layout, size);
        self.size = size;
    }

    pub fn clear(&self, encoder: &mut wgpu::CommandEncoder, clear_color: Vec3) {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("geometry_buffer_clear"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.inner.color.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear_color.x as f64,
                            g: clear_color.y as f64,
                            b: clear_color.z as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.inner.oit_accumulation.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.inner.oit_revealage.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.inner.depth.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
    }

    #[inline]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.inner.bind_group
    }

    fn opaque_attachments(&self) -> [Option<wgpu::RenderPassColorAttachment<'_>>; 1] {
        [Some(wgpu::RenderPassColorAttachment {
            view: &self.inner.color.view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })]
    }

    pub fn opaque_targets() -> &'static [Option<wgpu::ColorTargetState>] {
        &[Some(wgpu::ColorTargetState {
            format: Self::COLOR_FORMAT,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })]
    }

    fn additive_targets() -> &'static [Option<wgpu::ColorTargetState>] {
        &[Some(wgpu::ColorTargetState {
            format: Self::COLOR_FORMAT,
            blend: Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            }),
            write_mask: wgpu::ColorWrites::ALL,
        })]
    }

    fn alpha_attachments<'a>(&'a self) -> [Option<wgpu::RenderPassColorAttachment<'a>>; 2] {
        [
            Some(wgpu::RenderPassColorAttachment {
                view: &self.inner.oit_accumulation.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &self.inner.oit_revealage.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            }),
        ]
    }

    pub fn alpha_targets() -> &'static [Option<wgpu::ColorTargetState>] {
        &[
            Some(wgpu::ColorTargetState {
                format: Self::OIT_ACCUMULATION_FORMAT,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                }),
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: Self::OIT_REVEALAGE_FORMAT,
                blend: Some(wgpu::BlendState {
                    // = 0 * src + (1 - src_alpha) * dst  ==> multiplicative by (1 - α)
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::Zero,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::Zero,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                }),
                write_mask: wgpu::ColorWrites::ALL,
            }),
        ]
    }

    fn depth_stencil_state(
        depth_compare: wgpu::CompareFunction,
        depth_write_enabled: bool,
    ) -> wgpu::DepthStencilState {
        wgpu::DepthStencilState {
            format: Self::DEPTH_FORMAT,
            depth_write_enabled,
            depth_compare,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }
    }
}

impl GeometryBuffer {
    pub fn begin_opaque_render_pass<'rp>(
        &self,
        encoder: &'rp mut wgpu::CommandEncoder,
        label: &str,
    ) -> wgpu::RenderPass<'rp> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.inner.color.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.inner.depth.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }

    pub fn begin_alpha_render_pass<'rp>(
        &self,
        encoder: &'rp mut wgpu::CommandEncoder,
        label: &str,
    ) -> wgpu::RenderPass<'rp> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.inner.oit_accumulation.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.inner.oit_revealage.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.inner.depth.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }
}
