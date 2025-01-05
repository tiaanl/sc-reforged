use glam::UVec2;
use wgpu::include_wgsl;

use crate::RenderDevice;

/// Holds the various textures we render to for the final composite.
pub struct Compositor {
    /// Hold onto the device we are created with.
    device: RenderDevice,

    /// The dimensions of all the layers.
    pub size: UVec2,

    /// Texture to render albedo (color component) into.
    pub albedo_texture: wgpu::TextureView,
    /// Texture to render world position data into. Components represent (x, y, z, depth).
    pub position_texture: wgpu::TextureView,

    /// Holds all the textures and data we use for compositing.
    layers_bind_group: wgpu::BindGroup,
    /// For rendering the layers to the specified target.
    pipeline: wgpu::RenderPipeline,
    /// The format of the target we will render to.
    target_format: wgpu::TextureFormat,
}

impl Compositor {
    pub const ALBEDO_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
    pub const POSITION_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

    pub fn new(
        device: RenderDevice,
        size: UVec2,
        target_format: wgpu::TextureFormat,
        fog_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let albedo_texture = Self::create_albedo_texture(&device, size);
        let position_texture = Self::create_position_texture(&device, size);

        let layers_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("compositor_layers"),
                entries: &[
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
                ],
            });

        let layers_bind_group = Self::create_layers_bind_group(
            &device,
            &layers_bind_group_layout,
            &albedo_texture,
            &position_texture,
        );

        let pipeline = {
            let module = device.create_shader_module(include_wgsl!("compositor.wgsl"));

            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("compositor"),
                bind_group_layouts: &[&layers_bind_group_layout, fog_bind_group_layout],
                push_constant_ranges: &[],
            });

            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("compositor_layers"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target_format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            })
        };

        Self {
            device,
            size,
            albedo_texture,
            position_texture,
            layers_bind_group,
            pipeline,
            target_format,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.size = UVec2::new(width, height);

        self.albedo_texture = Self::create_texture(
            &self.device,
            "albedo",
            self.size,
            Self::ALBEDO_TEXTURE_FORMAT,
        );

        self.position_texture = Self::create_texture(
            &self.device,
            "position",
            self.size,
            Self::POSITION_TEXTURE_FORMAT,
        );

        let layers_bind_group_layout = &self.pipeline.get_bind_group_layout(0);

        self.layers_bind_group = Self::create_layers_bind_group(
            &self.device,
            layers_bind_group_layout,
            &self.albedo_texture,
            &self.position_texture,
        );
    }

    pub fn composite(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        fog_bind_group: &wgpu::BindGroup,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("compositor_layers"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.3,
                        g: 0.3,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.layers_bind_group, &[]);
        render_pass.set_bind_group(1, fog_bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }

    pub fn debug_panel(&mut self, egui: &egui::Context) {
        egui::Window::new("Compoositor")
            .default_open(false)
            .show(egui, |_ui| {});
    }

    #[inline]
    fn create_albedo_texture(device: &RenderDevice, size: UVec2) -> wgpu::TextureView {
        Self::create_texture(device, "albedo", size, Self::ALBEDO_TEXTURE_FORMAT)
    }

    #[inline]
    fn create_position_texture(device: &RenderDevice, size: UVec2) -> wgpu::TextureView {
        Self::create_texture(device, "position", size, Self::POSITION_TEXTURE_FORMAT)
    }

    fn create_texture(
        device: &RenderDevice,
        label: &str,
        size: UVec2,
        format: wgpu::TextureFormat,
    ) -> wgpu::TextureView {
        let extent = wgpu::Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn create_layers_bind_group(
        device: &RenderDevice,
        layout: &wgpu::BindGroupLayout,
        albedo_texture: &wgpu::TextureView,
        position_texture: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compositor_layers"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(albedo_texture),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(position_texture),
                },
            ],
        })
    }
}
