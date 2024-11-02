use std::sync::Arc;

struct DepthTexture {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
}

pub struct GpuTexture {
    #[allow(dead_code)]
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,

    depth_texture: DepthTexture,
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
    depth_buffer: bool,
    depth_compare_function: Option<wgpu::CompareFunction>,

    _phantom: std::marker::PhantomData<B>,
}

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
            depth_buffer: true,
            depth_compare_function: None,

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
        self.depth_buffer = false;
        self
    }

    pub fn depth_compare_function(mut self, depth_compare_function: wgpu::CompareFunction) -> Self {
        self.depth_compare_function = Some(depth_compare_function);
        self
    }
}

impl Renderer {
    const DEPTH_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn new(window: Arc<winit::window::Window>) -> Self {
        let winit::dpi::PhysicalSize { width, height } = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .expect("request adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::CLEAR_TEXTURE,
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
        // surface_config.present_mode = wgpu::PresentMode::AutoNoVsync;
        surface_config.present_mode = wgpu::PresentMode::AutoVsync;

        surface.configure(&device, &surface_config);

        let depth_texture = Self::create_depth_texture(&device, &surface_config);

        Self {
            device,
            queue,
            surface,
            surface_config,

            depth_texture,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;

        self.depth_texture = Self::create_depth_texture(&self.device, &self.surface_config);

        self.surface.configure(&self.device, &self.surface_config);
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
                    entry_point: config.vertex_entry.unwrap_or("vertex_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: B::vertex_buffers(),
                },
                primitive: config.primitive.unwrap_or_default(),
                depth_stencil: if config.depth_buffer {
                    self.depth_stencil_state(
                        config
                            .depth_compare_function
                            .unwrap_or(wgpu::CompareFunction::LessEqual),
                    )
                } else {
                    None
                },
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: config.shader_module,
                    entry_point: config.fragment_entry.unwrap_or("fragment_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: self.surface_config.format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            })
    }
}

/// Depth texture.
impl Renderer {
    fn create_depth_texture(
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> DepthTexture {
        let size = wgpu::Extent3d {
            width: surface_config.width.max(1),
            height: surface_config.height.max(1),
            depth_or_array_layers: 1,
        };

        let desc = wgpu::TextureDescriptor {
            label: Some("depth_texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        DepthTexture {
            _texture: texture,
            view,
            _sampler: sampler,
        }
    }

    pub fn depth_stencil_state(
        &self,
        depth_compare: wgpu::CompareFunction,
    ) -> Option<wgpu::DepthStencilState> {
        Some(wgpu::DepthStencilState {
            format: Self::DEPTH_TEXTURE_FORMAT,
            depth_write_enabled: true,
            depth_compare,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        })
    }

    pub fn render_pass_depth_stencil_attachment(
        &self,
        load: wgpu::LoadOp<f32>,
    ) -> Option<wgpu::RenderPassDepthStencilAttachment> {
        Some(wgpu::RenderPassDepthStencilAttachment {
            view: &self.depth_texture.view,
            depth_ops: Some(wgpu::Operations {
                load,
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        })
    }
}
