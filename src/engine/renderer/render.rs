use std::{
    ops::{Deref, Range},
    sync::Arc,
};

use glam::UVec2;
use wgpu::{PushConstantRange, util::DeviceExt};

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

    pub surface: super::surface::Surface,

    /// A bind group layout used for all texture bind groups.
    texture_bind_group_layout: wgpu::BindGroupLayout,

    mip_maps: MipMaps,
}

pub trait BufferLayout: Clone {
    fn layout() -> wgpu::VertexBufferLayout<'static>;
}

impl BufferLayout for () {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: 0,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[],
        }
    }
}

impl Renderer {
    const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

    pub fn new(window: Arc<winit::window::Window>) -> Self {
        let winit::dpi::PhysicalSize { width, height } = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window).expect("create surface");

        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            }))
            .expect("Could not request an adapter.");

        let features = adapter.features();

        let required_features =
            wgpu::Features::MULTI_DRAW_INDIRECT | wgpu::Features::PUSH_CONSTANTS;
        if !features.contains(required_features) {
            tracing::warn!("wgpu::Features::MULTI_DRAW_INDIRECT not available!");
        }

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

        let surface = super::surface::Surface::new(surface, surface_config);

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features,
                required_limits: wgpu::Limits {
                    // max_bind_groups: 5,
                    max_push_constant_size: 16,
                    // max_vertex_attributes: 24,
                    max_color_attachment_bytes_per_sample: 56,
                    ..Default::default()
                },
                ..Default::default()
            },
            None,
        ))
        .expect("request device");

        surface.configure(&device);

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
            texture_bind_group_layout,
            mip_maps,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let size = UVec2::new(width, height);
        self.surface.resize(&self.device, size);
    }

    pub fn create_vertex_buffer<B>(&self, label: &str, buffer: &[B]) -> wgpu::Buffer
    where
        B: bytemuck::NoUninit,
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
        let shader_module_label = format!("{label}_shader_module");
        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&shader_module_label),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(source)),
            })
    }

    #[must_use]
    pub fn build_render_pipeline<'a, B>(
        &'a self,
        label: &'a str,
        module: &'a wgpu::ShaderModule,
    ) -> RenderPipelineBuilder<'a, B>
    where
        B: BufferLayout,
    {
        RenderPipelineBuilder {
            renderer: self,
            label,
            bindings: vec![],
            push_constants: vec![],
            module,
            color_target_format: None,
            primitive_state: None,
            depth_compare: None,
            depth_writes: true,
            blend: None,
            vertex_entry: None,
            fragment_entry: None,
            _phantom: std::marker::PhantomData,
        }
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
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::default(),
                aspect: wgpu::TextureAspect::All,
            },
            image,
            wgpu::TexelCopyBufferLayout {
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

/// A single object passed around during the rendering of a single frame.
pub struct Frame<'r> {
    pub device: RenderDevice,
    pub queue: RenderQueue,

    /// The encoder to use for creating render passes.
    pub encoder: wgpu::CommandEncoder,

    /// The window surface.
    pub surface: wgpu::TextureView,

    /// The [Renderer] we belong to.
    pub renderer: &'r Renderer,
}

pub struct RenderPipelineBuilder<'a, V>
where
    V: BufferLayout,
{
    renderer: &'a Renderer,

    label: &'a str,

    bindings: Vec<&'a wgpu::BindGroupLayout>,

    push_constants: Vec<PushConstantRange>,

    module: &'a wgpu::ShaderModule,
    color_target_format: Option<wgpu::TextureFormat>,

    /// A specific primitive state, otherwise use the default.
    primitive_state: Option<wgpu::PrimitiveState>,

    /// Use depth testing in the pipeline.
    depth_compare: Option<wgpu::CompareFunction>,
    depth_writes: bool,

    /// Blend state.
    blend: Option<wgpu::BlendState>,

    /// Fragment shader entry point.
    fragment_entry: Option<&'a str>,

    /// Vertex shader entry point.
    vertex_entry: Option<&'a str>,

    _phantom: std::marker::PhantomData<V>,
}

impl<'a, V> RenderPipelineBuilder<'a, V>
where
    V: BufferLayout,
{
    pub fn with_primitive(mut self, primitive_state: wgpu::PrimitiveState) -> Self {
        self.primitive_state = Some(primitive_state);
        self
    }

    pub fn with_depth_compare(mut self, compare: wgpu::CompareFunction) -> Self {
        self.depth_compare = Some(compare);
        self
    }

    #[allow(unused)]
    pub fn with_depth_writes(mut self, depth_writes: bool) -> Self {
        self.depth_writes = depth_writes;
        self
    }

    pub fn binding(mut self, layout: &'a wgpu::BindGroupLayout) -> Self {
        self.bindings.push(layout);
        self
    }

    pub fn push_constant(mut self, stages: wgpu::ShaderStages, range: Range<u32>) -> Self {
        self.push_constants
            .push(wgpu::PushConstantRange { stages, range });
        self
    }

    pub fn with_vertex_entry(mut self, entry: &'a str) -> Self {
        self.vertex_entry = Some(entry);
        self
    }

    pub fn with_fragment_entry(mut self, entry: &'a str) -> Self {
        self.fragment_entry = Some(entry);
        self
    }

    pub fn with_blend(mut self, blend: wgpu::BlendState) -> Self {
        self.blend = Some(blend);
        self
    }

    pub fn build(self) -> wgpu::RenderPipeline {
        let layout = self
            .renderer
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(self.label),
                bind_group_layouts: &self.bindings,
                push_constant_ranges: &self.push_constants,
            });

        self.renderer
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(self.label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: self.module,
                    entry_point: self.vertex_entry,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[V::layout()],
                },
                primitive: self.primitive_state.unwrap_or_default(),
                depth_stencil: self.depth_compare.map(|comp| wgpu::DepthStencilState {
                    // TODO: Pass in the depth texture format?
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: comp,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: self.module,
                    entry_point: self.fragment_entry,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        // Use the color target format specified, otherwise use the format of the
                        // window surface.
                        format: self
                            .color_target_format
                            .unwrap_or(self.renderer.surface.format()),
                        blend: self.blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            })
    }
}
