use std::num::NonZeroU32;

use ahash::HashMap;
use glam::UVec2;

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
    pub texture_data_index: u32,
}

/// A store/cache for textures used by the [super::ModelRenderer].
pub struct RenderTextures {
    /// All [RenderTexture]s we're keeping track of.
    textures: Storage<RenderTexture>,
    /// Cached for Image->RenderTexture.
    image_to_render_texture: HashMap<Handle<Image>, Handle<RenderTexture>>,
    /// A bucket holds textures at a specified size.
    buckets: Vec<Bucket>,
    /// A Buffer to write all texture information into that is required by shaders to use the
    /// correct texture.
    texture_data_buffer: wgpu::Buffer,
    /// The amount of [gpu::TextureData] in the buffer.
    texture_data_count: u32,
    /// Bind group layout for `texture_data_bind_group`.
    pub texture_data_bind_group_layout: wgpu::BindGroupLayout,
    /// Bind group that has all the relevant data for shaders to get texture data.
    pub texture_data_bind_group: wgpu::BindGroup,
}

impl RenderTextures {
    const FIRST_POW: u32 = 4; // 16
    const MAX_POW: u32 = 9; // 512

    pub fn new() -> Self {
        let sampler = renderer().device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("model_renderer_sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let buckets: Vec<Bucket> = (0..=(Self::MAX_POW - Self::FIRST_POW))
            .map(|i| Bucket::new(Self::calculate_bucket_size(i as usize)))
            .collect();

        let texture_data_buffer = {
            let initial_size = buckets.len()
                * Bucket::INITIAL_LAYER_COUNT as usize
                * std::mem::size_of::<gpu::TextureData>();

            tracing::info!("Allocating texture data buffer of {} bytes.", initial_size);

            renderer().device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("texture_data"),
                size: initial_size as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        };

        let texture_data_bind_group_layout =
            renderer()
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("texture_data_bind_group_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2Array,
                                multisampled: false,
                            },
                            count: Some(NonZeroU32::new(buckets.len() as u32).unwrap()),
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let texture_data_bind_group = {
            let texture_views: Vec<_> = buckets.iter().map(|b| &b.texture_view).collect();
            renderer()
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("texture_data_bind_group"),
                    layout: &texture_data_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureViewArray(&texture_views),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: texture_data_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                    ],
                })
        };

        Self {
            textures: Storage::default(),
            image_to_render_texture: HashMap::default(),
            buckets,
            texture_data_buffer,
            texture_data_count: 0,
            texture_data_bind_group_layout,
            texture_data_bind_group,
        }
    }

    #[inline]
    pub fn get(&self, handle: Handle<RenderTexture>) -> Option<&RenderTexture> {
        self.textures.get(handle)
    }

    pub fn get_or_create(&mut self, image_handle: Handle<Image>) -> Handle<RenderTexture> {
        if let Some(render_texture) = self.image_to_render_texture.get(&image_handle) {
            return *render_texture;
        };

        let image = images()
            .get(image_handle)
            .expect("Adding image that doesn't exist!");

        let bucket_index = Self::calculate_bucket_index(image.size);
        let bucket = &mut self.buckets[bucket_index];
        let layer = bucket.insert(image);

        // Write the texture data into the buffer.
        let texture_data_index = {
            let index = self.texture_data_count;
            self.texture_data_count += 1;

            let texture_data = gpu::TextureData {
                bucket: bucket_index as u32,
                layer,
            };
            let offset = index as u64 * std::mem::size_of::<gpu::TextureData>() as u64;
            renderer().queue.write_buffer(
                &self.texture_data_buffer,
                offset as u64,
                bytemuck::bytes_of(&texture_data),
            );

            index
        };

        {
            let size = wgpu::Extent3d {
                width: image.size.x,
                height: image.size.y,
                depth_or_array_layers: 1,
            };

            let texture = renderer().device.create_texture(&wgpu::TextureDescriptor {
                label: Some(&format!("texture_{image_handle}")),
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

            let texture = RenderTexture {
                blend_mode: image.blend_mode,
                texture_data_index,
            };

            let render_texture = self.textures.insert(texture);

            // Cache the result.
            self.image_to_render_texture
                .insert(image_handle, render_texture);

            render_texture
        }
    }

    #[inline]
    fn calculate_bucket_index(image_size: UVec2) -> usize {
        let max_size = image_size.x.max(image_size.y);
        debug_assert!(max_size > 0, "texture size must be > 0");

        // Ceil to next power of two.
        let size = max_size.next_power_of_two();

        // Convert to exponent, clamp, then offset by FIRST_POW.
        let p = size.ilog2();
        let p = p.clamp(Self::FIRST_POW, Self::MAX_POW);

        (p - Self::FIRST_POW) as usize
    }

    #[inline]
    fn calculate_bucket_size(index: usize) -> u32 {
        2_u32.pow(Self::FIRST_POW + index as u32)
    }
}

struct Bucket {
    /// The size of the textures stored in this bucket.  E.g. 16, 32, 64, etc.
    _size: u32,
    /// The texture holding the data.
    texture: wgpu::Texture,
    /// A view that can be bound and will have access to all the textures/layers in this bucket.
    texture_view: wgpu::TextureView,
    /// Total number of layers available in this bucket.
    layer_capacity: u32,
    /// Next available layer.
    next_layer: u32,
}

impl Bucket {
    const INITIAL_LAYER_COUNT: u32 = 256;

    fn new(size: u32) -> Self {
        let label = format!("texture_bucket_{size}");

        let texture = renderer().device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&label),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: Self::INITIAL_LAYER_COUNT,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let layer_capacity = Self::INITIAL_LAYER_COUNT;

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(&format!("{label}_view")),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            base_array_layer: 0,
            array_layer_count: Some(Self::INITIAL_LAYER_COUNT),
            ..Default::default()
        });

        Self {
            _size: size,
            texture,
            texture_view,
            layer_capacity,
            next_layer: 0,
        }
    }

    /// Insert the image into the first available layer and return the index to the layer.
    fn insert(&mut self, image: &Image) -> u32 {
        if self.next_layer >= self.layer_capacity {
            panic!("Too many textures!");
        }

        let layer = self.next_layer;
        self.next_layer += 1;

        renderer().queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: layer,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &image.data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(image.size.x * 4),
                rows_per_image: Some(image.size.y),
            },
            wgpu::Extent3d {
                width: image.size.x,
                height: image.size.y,
                depth_or_array_layers: 1,
            },
        );

        layer
    }
}

mod gpu {
    use bytemuck::NoUninit;

    #[derive(Clone, Copy, NoUninit)]
    #[repr(C)]
    pub struct TextureData {
        pub bucket: u32,
        pub layer: u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_and_clamp() {
        assert_eq!(RenderTextures::calculate_bucket_index(UVec2::splat(8)), 0);
        assert_eq!(RenderTextures::calculate_bucket_index(UVec2::splat(16)), 0);

        assert_eq!(RenderTextures::calculate_bucket_index(UVec2::splat(15)), 0);
        assert_eq!(RenderTextures::calculate_bucket_index(UVec2::splat(17)), 1);
    }

    #[test]
    fn bucket_indices() {
        assert_eq!(RenderTextures::calculate_bucket_size(0), 16);
        assert_eq!(RenderTextures::calculate_bucket_size(1), 32);
        assert_eq!(RenderTextures::calculate_bucket_size(2), 64);
        assert_eq!(RenderTextures::calculate_bucket_size(3), 128);
        assert_eq!(RenderTextures::calculate_bucket_size(4), 256);
        assert_eq!(RenderTextures::calculate_bucket_size(5), 512);

        assert_eq!(RenderTextures::calculate_bucket_index(UVec2::splat(16)), 0);
        assert_eq!(RenderTextures::calculate_bucket_index(UVec2::splat(32)), 1);
        assert_eq!(RenderTextures::calculate_bucket_index(UVec2::splat(64)), 2);
        assert_eq!(RenderTextures::calculate_bucket_index(UVec2::splat(128)), 3);
        assert_eq!(RenderTextures::calculate_bucket_index(UVec2::splat(256)), 4);
        assert_eq!(RenderTextures::calculate_bucket_index(UVec2::splat(512)), 5);
    }
}
