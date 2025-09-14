use ahash::{HashMap, HashMapExt};

use crate::{
    engine::{prelude::Frame, storage::Handle},
    game::{
        geometry_buffers::GeometryBuffers,
        image::{Image, images},
    },
    wgsl_shader,
};

pub struct SkyRenderer {
    /// Textures that can be used to render the sky slices.
    textures: HashMap<i32, wgpu::TextureView>,
    /// Pipeline to render the sky slices with.
    pipeline: wgpu::RenderPipeline,
}

impl SkyRenderer {
    pub fn new(device: &wgpu::Device, camera_bind_group_layout: &wgpu::BindGroupLayout) -> Self {
        let pipeline = {
            let module = device.create_shader_module(wgsl_shader!("sky"));

            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("sky"),
                bind_group_layouts: &[camera_bind_group_layout],
                push_constant_ranges: &[],
            });

            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("sky"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    strip_index_format: Some(wgpu::IndexFormat::Uint32),
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: GeometryBuffers::opaque_targets(),
                }),
                multiview: None,
                cache: None,
            })
        };

        Self {
            textures: HashMap::with_capacity(8),
            pipeline,
        }
    }

    pub fn set_sky_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        index: i32,
        image: Handle<Image>,
    ) {
        let Some(image) = images().get(image) else {
            tracing::warn!("Sky image nout found!");
            return;
        };

        let view = Self::create_texture(device, queue, index, image);

        self.textures.insert(index, view);
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        geometry_buffers: &GeometryBuffers,
        camera_bind_group: &wgpu::BindGroup,
    ) {
        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sky"),
                color_attachments: &geometry_buffers.opaque_attachments(),
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }

    fn create_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        index: i32,
        image: &Image,
    ) -> wgpu::TextureView {
        let size = wgpu::Extent3d {
            width: image.size.x,
            height: image.size.y,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("sky_{index}")),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::default(),
                aspect: wgpu::TextureAspect::All,
            },
            &image.data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(image.size.x * 4),
                rows_per_image: Some(image.size.y),
            },
            size,
        );

        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }
}
