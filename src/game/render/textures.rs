use std::sync::{Arc, RwLock};

use glam::UVec2;

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

    textures: RwLock<StorageMap<Handle<Image>, Texture, Arc<TextureData>>>,
}

impl Textures {
    pub fn new(render_context: RenderContext, images: Arc<Images>) -> Self {
        Self {
            render_context,
            images,

            textures: RwLock::new(StorageMap::default()),
        }
    }

    pub fn images(&self) -> Arc<Images> {
        Arc::clone(&self.images)
    }

    pub fn get(&self, handle: Handle<Texture>) -> Option<Arc<TextureData>> {
        let textures = self.textures.read().unwrap();
        textures.get(handle).cloned()
    }

    /// Returns a texture handle that covers the full source image.
    pub fn create_from_image(&self, image: Handle<Image>) -> Option<Handle<Texture>> {
        {
            let textures = self.textures.read().unwrap();
            if let Some(handle) = textures.get_handle_by_key(&image) {
                return Some(handle);
            }
        }

        let image_handle = image;
        let image = self.images.get(image)?;

        let view = self.create_texture_internal(&image.data);

        let handle = {
            let mut textures = self.textures.write().unwrap();
            textures.insert(
                image_handle,
                Arc::new(TextureData {
                    _image: image_handle,
                    size: image.size,
                    view,
                }),
            )
        };

        Some(handle)
    }

    fn create_texture_internal(&self, image: &image::RgbaImage) -> wgpu::TextureView {
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
    /// Size of the full texture in pixels.
    pub size: UVec2,
    /// The [wgpu::TextureView] used to access this texture during rendering.
    pub view: wgpu::TextureView,
}
