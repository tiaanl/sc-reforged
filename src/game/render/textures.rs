use std::sync::Arc;

use crate::{
    engine::{
        renderer::RenderContext,
        storage::{Handle, StorageMap},
    },
    game::assets::{image::Image, images::Images},
};

pub struct Texture;

pub struct Textures {
    render_context: RenderContext,
    images: Arc<Images>,

    textures: StorageMap<Handle<Image>, Texture, TextureData>,
}

impl Textures {
    pub fn new(render_context: RenderContext, images: Arc<Images>) -> Self {
        Self {
            render_context,
            images,

            textures: StorageMap::default(),
        }
    }

    pub fn get(&self, handle: Handle<Texture>) -> Option<&TextureData> {
        self.textures.get(handle)
    }

    pub fn create(&mut self, image: Handle<Image>) -> Option<Handle<Texture>> {
        if let Some(handle) = self.textures.get_handle_by_key(&image) {
            return Some(handle);
        }

        let RenderContext { device, queue } = &self.render_context;

        let image_handle = image;
        let image = self.images.get(image)?;

        let size = wgpu::Extent3d {
            width: image.size.x,
            height: image.size.y,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
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
                origin: wgpu::Origin3d::ZERO,
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

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Some(self.textures.insert(image_handle, TextureData { view }))
    }
}

pub struct TextureData {
    pub view: wgpu::TextureView,
}
