use std::{ops::Deref, sync::Arc};

use wgpu::util::DeviceExt;

use crate::DepthBuffer;

use super::mip_maps::MipMaps;

#[derive(Clone)]
pub struct RenderDevice(Arc<wgpu::Device>);

impl From<wgpu::Device> for RenderDevice {
    fn from(value: wgpu::Device) -> Self {
        Self(Arc::new(value))
    }
}

impl Deref for RenderDevice {
    type Target = wgpu::Device;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

#[derive(Clone)]
pub struct RenderQueue(Arc<wgpu::Queue>);

impl From<wgpu::Queue> for RenderQueue {
    fn from(value: wgpu::Queue) -> Self {
        Self(Arc::new(value))
    }
}

impl Deref for RenderQueue {
    type Target = wgpu::Queue;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

pub struct Renderer {
    pub device: RenderDevice,
    pub queue: RenderQueue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,

    pub depth_buffer: Arc<DepthBuffer>,

    /// A bind group layout used for all texture bind groups.
    texture_bind_group_layout: wgpu::BindGroupLayout,

    mip_maps: MipMaps,
}

pub trait BufferLayout: Sized {
    fn vertex_buffers() -> &'static [wgpu::VertexBufferLayout<'static>];
}

impl BufferLayout for () {
    fn vertex_buffers() -> &'static [wgpu::VertexBufferLayout<'static>] {
        &[wgpu::VertexBufferLayout {
            array_stride: 0,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[],
        }]
    }
}

pub struct RenderPipelineConfig<'a, B>
where
    B: BufferLayout,
{
    label: &'a str,
    shader_module: &'a wgpu::ShaderModule,
    bind_group_layouts: Vec<&'a wgpu::BindGroupLayout>,

    vertex_entry: Option<&'a str>,
    fragment_entry: Option<&'a str>,
    primitive: Option<wgpu::PrimitiveState>,
    use_depth_buffer: bool,
    depth_compare_function: Option<wgpu::CompareFunction>,
    blend_state: Option<wgpu::BlendState>,

    _phantom: std::marker::PhantomData<B>,
}

#[allow(unused)]
impl<'a, B> RenderPipelineConfig<'a, B>
where
    B: BufferLayout,
{
    pub fn new(label: &'a str, shader_module: &'a wgpu::ShaderModule) -> Self {
        Self {
            label,
            shader_module,
            bind_group_layouts: vec![],

            vertex_entry: None,
            fragment_entry: None,
            primitive: None,
            use_depth_buffer: true,
            depth_compare_function: None,
            blend_state: None,

            _phantom: std::marker::PhantomData::<B>,
        }
    }

    pub fn bind_group_layout(mut self, bind_group_layout: &'a wgpu::BindGroupLayout) -> Self {
        self.bind_group_layouts.push(bind_group_layout);
        self
    }

    pub fn vertex_entry(mut self, entry: &'a str) -> Self {
        self.vertex_entry = Some(entry);
        self
    }

    pub fn fragment_entry(mut self, entry: &'a str) -> Self {
        self.fragment_entry = Some(entry);
        self
    }

    pub fn primitive(mut self, primitive: wgpu::PrimitiveState) -> Self {
        self.primitive = Some(primitive);
        self
    }

    pub fn disable_depth_buffer(mut self) -> Self {
        self.use_depth_buffer = false;
        self
    }

    pub fn blend_state(mut self, blend_state: wgpu::BlendState) -> Self {
        self.blend_state = Some(blend_state);
        self
    }

    // pub fn depth_compare_function(mut self, depth_compare_function: wgpu::CompareFunction) -> Self {
    //     self.depth_compare_function = Some(depth_compare_function);
    //     self
    // }
}

impl Renderer {
    const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

    pub fn new(window: Arc<winit::window::Window>) -> Self {
        let winit::dpi::PhysicalSize { width, height } = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .expect("request adapter");

        let features = adapter.features();
        if !features.contains(wgpu::Features::MULTI_DRAW_INDIRECT) {
            tracing::warn!("wgpu::Features::MULTI_DRAW_INDIRECT not available!");
        }

        let required_features = wgpu::Features::MULTI_DRAW_INDIRECT;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features,
                required_limits: wgpu::Limits {
                    max_bind_groups: 5,
                    ..Default::default()
                },
                ..Default::default()
            },
            None,
        ))
        .expect("request device");

        let surface = instance.create_surface(window).expect("create surface");

        let surface_caps = surface.get_capabilities(&adapter);

        // Find a sRGB surface format or use the first.
        let format = surface_caps
            .formats
            .iter()
            .find(|cap| cap.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let mut surface_config = surface
            .get_default_config(&adapter, width, height)
            .expect("surface get default configuration");
        surface_config.format = format;
        surface_config.present_mode = wgpu::PresentMode::AutoNoVsync;
        // surface_config.present_mode = wgpu::PresentMode::AutoVsync;

        surface.configure(&device, &surface_config);

        let depth_texture = Arc::new(DepthBuffer::new(&device, &surface_config));

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture_bind_group_layout"),
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
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let mip_maps = MipMaps::new(&device, &texture_bind_group_layout, Self::TEXTURE_FORMAT);

        Self {
            device: device.into(),
            queue: queue.into(),
            surface,
            surface_config,
            depth_buffer: depth_texture,
            texture_bind_group_layout,
            mip_maps,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.depth_buffer = Arc::new(DepthBuffer::new(&self.device, &self.surface_config));
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn width(&self) -> u32 {
        self.surface_config.width
    }

    pub fn height(&self) -> u32 {
        self.surface_config.height
    }

    pub fn create_vertex_buffer<B>(&self, label: &str, buffer: &[B]) -> wgpu::Buffer
    where
        B: BufferLayout + bytemuck::NoUninit,
    {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(buffer),
                usage: wgpu::BufferUsages::VERTEX,
            })
    }

    pub fn create_index_buffer(&self, label: &str, buffer: &[u32]) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(buffer),
                usage: wgpu::BufferUsages::INDEX,
            })
    }

    pub fn create_shader_module(&self, label: &str, source: &str) -> wgpu::ShaderModule {
        let shader_module_label = format!("{}_shader_module", label);
        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&shader_module_label),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(source)),
            })
    }

    pub fn create_render_pipeline<B>(&self, config: RenderPipelineConfig<B>) -> wgpu::RenderPipeline
    where
        B: BufferLayout,
    {
        let layout_label = format!("{}_pipeline_layout", config.label);
        let layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(&layout_label),
                bind_group_layouts: &config.bind_group_layouts,
                push_constant_ranges: &[],
            });

        let pipeline_label = format!("{}_render_pipeline", config.label);
        self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(&pipeline_label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: config.shader_module,
                    entry_point: config.vertex_entry,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: B::vertex_buffers(),
                },
                primitive: config.primitive.unwrap_or_default(),
                depth_stencil: if config.use_depth_buffer {
                    Some(
                        self.depth_buffer.depth_stencil_state(
                            config
                                .depth_compare_function
                                .unwrap_or(wgpu::CompareFunction::Less),
                            true,
                        ),
                    )
                } else {
                    None
                },
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: config.shader_module,
                    entry_point: config.fragment_entry,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: self.surface_config.format,
                        blend: config.blend_state,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            })
    }

    pub fn create_texture_view(&self, label: &str, image: &image::RgbaImage) -> wgpu::TextureView {
        let (width, height) = (image.width(), image.height());

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let mip_level_count = (width.max(height) as f32).log2().floor() as u32 + 1;

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::default(),
                aspect: wgpu::TextureAspect::All,
            },
            image,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            size,
        );

        self.mip_maps
            .generate_mip_maps(&self.device, &self.queue, &texture, mip_level_count);

        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub fn texture_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.texture_bind_group_layout
    }

    pub fn create_texture_bind_group(
        &self,
        label: &str,
        texture_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    pub fn create_sampler(
        &self,
        label: &str,
        address_mode: wgpu::AddressMode,
        mag_filter: wgpu::FilterMode,
        min_filter: wgpu::FilterMode,
    ) -> wgpu::Sampler {
        self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(label),
            address_mode_u: address_mode,
            address_mode_v: address_mode,
            address_mode_w: address_mode,
            mag_filter,
            min_filter,
            ..Default::default()
        })
    }
}

/// Depth texture.
pub struct Frame {
    pub device: RenderDevice,
    pub queue: RenderQueue,
    pub depth_buffer: Arc<DepthBuffer>,

    /// The encoder to use for creating render passes.
    pub encoder: wgpu::CommandEncoder,
    /// The window surface.
    pub surface: wgpu::TextureView,
}

impl Frame {
    pub fn clear_color_and_depth(&mut self, color: wgpu::Color, depth: f32) {
        // Creating and dropping the render pass will clear the buffers.
        self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("world_clear_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.surface,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_buffer.texture_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(depth),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
    }

    pub fn begin_basic_render_pass(&mut self, label: &str, depth_test: bool) -> wgpu::RenderPass {
        self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.surface,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: if depth_test {
                Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_buffer.texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                })
            } else {
                None
            },
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }
}
