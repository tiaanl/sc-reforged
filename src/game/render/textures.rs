use std::{path::Path, sync::Arc};

use glam::UVec2;

use crate::{
    engine::{
        assets::AssetError,
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

    pub fn load(&mut self, path: &Path) -> Result<Handle<Texture>, AssetError> {
        let image_handle = self.images.load(path)?;
        // SAFETY: We can unwrap here, because the only reason
        //         `create_from_image` can fail is if the image handle is not
        //         found, but we just created it here, so no error is expected.
        Ok(self.create_from_image(image_handle).unwrap())
    }

    pub fn create_from_image(&mut self, image: Handle<Image>) -> Option<Handle<Texture>> {
        if let Some(handle) = self.textures.get_handle_by_key(&image) {
            return Some(handle);
        }

        let image_handle = image;
        let image = self.images.get(image)?;

        let view = self.create_texture_internal(&image.data);

        Some(self.textures.insert(
            image_handle,
            TextureData {
                _image: image_handle,
                size: image.size,
                view,
            },
        ))
    }

    fn create_texture_internal(&mut self, image: &image::RgbaImage) -> wgpu::TextureView {
        let RenderContext { device, queue } = &self.render_context;

        let (width, height) = (image.width(), image.height());

        let size = wgpu::Extent3d {
            width,
            height,
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
            image,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            size,
        );

        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }
}

pub struct TextureData {
    /// The image used to create this texture.
    pub _image: Handle<Image>,
    /// Size of the texture in pixels.
    pub size: UVec2,
    /// The [wgpu::TextureView] used to access this texture during rendering.
    pub view: wgpu::TextureView,
}
