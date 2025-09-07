use std::num::NonZeroU32;

use crate::{
    engine::{
        prelude::renderer,
        storage::{Handle, Storage},
    },
    game::image::{BlendMode, Image, images},
};

/// A texture that can be use for rendering.
pub struct RenderTexture {
    pub _blend_mode: BlendMode,
    pub texture_view: wgpu::TextureView,
}

pub struct RenderTextureSet {
    pub bind_group: wgpu::BindGroup,
}

/// A store/cache for textures used by the [super::ModelRenderer].
pub struct RenderTextures {
    /// All [RenderTexture]s we're keeping track of.
    textures: Storage<RenderTexture>,
    /// A [wgpu::TextureView] that can be used to pad the texture arrays. Just a single red pixel.
    dummy_texture_view: wgpu::TextureView,
    /// Bind group layuout to use for a [RenderTextureSet].
    pub texture_set_bind_group_layout: wgpu::BindGroupLayout,
    /// The [wgpu::Sampler] we use for all textures.
    sampler: wgpu::Sampler,
}

impl RenderTextures {
    const MAX_TEXTURES_PER_SET: u32 = 8;

    pub fn new() -> Self {
        let dummy_texture_view = create_dummy_texture_view();

        let texture_set_bind_group_layout =
            renderer()
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("model_renderer_texture_set_bind_group_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: NonZeroU32::new(Self::MAX_TEXTURES_PER_SET),
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
            dummy_texture_view,
            texture_set_bind_group_layout,
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

        let texture = RenderTexture {
            _blend_mode: image.blend_mode,
            texture_view,
        };

        self.textures.insert(texture)
    }

    pub fn create_texture_set(&mut self, textures: Vec<Handle<RenderTexture>>) -> RenderTextureSet {
        if textures.len() > Self::MAX_TEXTURES_PER_SET as usize {
            tracing::warn!(
                "Texture set can only hold {} textures! ({} given)",
                Self::MAX_TEXTURES_PER_SET,
                textures.len()
            );
        }

        let texture_views = {
            let mut result = textures
                .iter()
                .map(|texture| {
                    self.textures
                        .get(*texture)
                        .map(|texture| &texture.texture_view)
                        .unwrap_or(&self.dummy_texture_view)
                })
                .collect::<Vec<_>>();

            result.resize(
                Self::MAX_TEXTURES_PER_SET as usize,
                &self.dummy_texture_view,
            );

            result
        };

        let bind_group = renderer()
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("model_renderer_texture_set"),
                layout: &self.texture_set_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureViewArray(&texture_views),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });

        RenderTextureSet { bind_group }
    }

    #[inline]
    pub fn _get(&self, texture_handle: Handle<RenderTexture>) -> Option<&RenderTexture> {
        self.textures.get(texture_handle)
    }
}

fn create_dummy_texture_view() -> wgpu::TextureView {
    let size = wgpu::Extent3d {
        width: 1,
        height: 1,
        depth_or_array_layers: 1,
    };

    let texture = renderer().device.create_texture(&wgpu::TextureDescriptor {
        label: Some("dummy"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    let data = [255_u8, 0, 0, 255];

    renderer().queue.write_texture(
        wgpu::TexelCopyTextureInfoBase {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        &data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        size,
    );

    texture.create_view(&wgpu::TextureViewDescriptor::default())
}
