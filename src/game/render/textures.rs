use std::{path::Path, sync::Arc};

use glam::{UVec2, Vec2};

use crate::{
    engine::{
        assets::AssetError,
        renderer::RenderContext,
        storage::{Handle, StorageMap},
    },
    game::assets::{image::Image, images::Images},
};

pub struct Texture;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum TextureKey {
    Image(Handle<Image>),
    Area {
        image: Handle<Image>,
        pos: UVec2,
        size: UVec2,
    },
}

pub struct Textures {
    render_context: RenderContext,
    images: Arc<Images>,

    textures: StorageMap<TextureKey, Texture, TextureData>,
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

    /// Returns a texture handle that covers the full source image.
    pub fn create_from_image(&mut self, image: Handle<Image>) -> Option<Handle<Texture>> {
        let key = TextureKey::Image(image);
        if let Some(handle) = self.textures.get_handle_by_key(&key) {
            return Some(handle);
        }

        let image_handle = image;
        let image = self.images.get(image)?;

        let view = self.create_texture_internal(&image.data);

        Some(self.textures.insert(
            key,
            TextureData {
                _image: image_handle,
                size: image.size,
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
                view,
            },
        ))
    }

    /// Returns a texture handle for a sub-rectangle of an image.
    pub fn create_from_sub_image(
        &mut self,
        image: Handle<Image>,
        pos: UVec2,
        size: UVec2,
    ) -> Option<Handle<Texture>> {
        let key = TextureKey::Area { image, pos, size };
        if let Some(handle) = self.textures.get_handle_by_key(&key) {
            return Some(handle);
        }

        let base_texture = self.create_from_image(image)?;
        let image_handle = image;
        let image = self.images.get(image)?;
        let view = self.textures.get(base_texture)?.view.clone();

        if size.x == 0 || size.y == 0 {
            return None;
        }
        let bottom_right = pos.checked_add(size)?;
        if bottom_right.x > image.size.x || bottom_right.y > image.size.y {
            return None;
        }

        let image_size = image.size.as_vec2();
        let uv_min = pos.as_vec2() / image_size;
        let uv_max = bottom_right.as_vec2() / image_size;

        Some(self.textures.insert(
            key,
            TextureData {
                _image: image_handle,
                size,
                uv_min,
                uv_max,
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
    /// Lower-left UV bound used when sampling this texture.
    pub uv_min: Vec2,
    /// Upper-right UV bound used when sampling this texture.
    pub uv_max: Vec2,
    /// The [wgpu::TextureView] used to access this texture during rendering.
    pub view: wgpu::TextureView,
}
