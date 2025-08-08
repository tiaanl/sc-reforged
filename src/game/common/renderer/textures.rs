use crate::{
    engine::{
        prelude::renderer,
        storage::{Handle, Storage},
    },
    game::image::{BlendMode, Image, images},
};

/// A texture that can be use for rendering.
pub struct RenderTexture {
    pub blend_mode: BlendMode,
    pub bind_group: wgpu::BindGroup,
}

/// A store/cache for textures used by the [super::ModelRenderer].
pub struct Textures {
    textures: Storage<RenderTexture>,

    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl Textures {
    pub fn new() -> Self {
        let texture_bind_group_layout =
            renderer()
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("model_renderer_texture_bind_group_layout"),
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

        let sampler = renderer().device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("model_renderer_sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            textures: Storage::default(),
            texture_bind_group_layout,
            sampler,
        }
    }

    pub fn add(&mut self, image_handle: Handle<Image>) -> Handle<RenderTexture> {
        let image = images()
            .get(image_handle)
            .expect("Adding image that doesn't exist!");

        let size = wgpu::Extent3d {
            width: image.size.x,
            height: image.size.y,
            depth_or_array_layers: 1,
        };

        let texture = renderer().device.create_texture(&wgpu::TextureDescriptor {
            label: Some("texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        renderer().queue.write_texture(
            wgpu::TexelCopyTextureInfoBase {
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

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = renderer()
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("model_renderer_bind_group"),
                layout: &self.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });

        let texture = RenderTexture {
            blend_mode: image.blend_mode,
            bind_group,
        };

        self.textures.insert(texture)
    }

    #[inline]
    pub fn get(&self, texture_handle: Handle<RenderTexture>) -> Option<&RenderTexture> {
        self.textures.get(texture_handle)
    }
}
