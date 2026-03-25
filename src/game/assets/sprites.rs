use std::{ops::RangeInclusive, path::PathBuf, sync::Arc};

use crate::{
    engine::storage::{Handle, StorageMap},
    game::{
        assets::{image::Image, images::Images},
        config::ImageDefs,
    },
};

#[derive(Debug)]
pub struct ColorKeyRange {
    r: RangeInclusive<f32>,
    g: RangeInclusive<f32>,
    b: RangeInclusive<f32>,
}

impl Default for ColorKeyRange {
    fn default() -> Self {
        // Purple: rgb(255 0 255)
        Self {
            r: 1.0..=1.0,
            g: 0.0..=0.0,
            b: 1.0..=1.0,
        }
    }
}

#[derive(Debug)]
pub struct SpriteFrame {
    top_left: glam::UVec2,
    bottom_right: glam::UVec2,
}

#[derive(Debug)]
pub struct Sprite3d {
    pub name: String,
    pub image: Handle<Image>,
    pub size: glam::UVec2,
    pub alpha: Option<f32>,
    pub color_key_range: Option<ColorKeyRange>,
    pub frames: Vec<SpriteFrame>,
}

pub struct Sprites {
    images: Arc<Images>,
    sprites: StorageMap<String, Sprite3d>,
}

impl Sprites {
    pub fn new(images: Arc<Images>) -> Self {
        Self {
            images,
            sprites: StorageMap::default(),
        }
    }

    #[inline]
    pub fn get(&self, handle: Handle<Sprite3d>) -> Option<&Sprite3d> {
        self.sprites.get(handle)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&Sprite3d> {
        self.sprites.get_by_key(&String::from(name))
    }

    pub fn load_image_defs(&mut self, image_defs: &ImageDefs) {
        /*
        for s in image_defs.sprite_3d.iter() {
            let image_path = PathBuf::from("textures")
                .join("object")
                .join(&s.texture_name);

            let image_handle = match self.images.load(&image_path) {
                Ok(image_handle) => image_handle,
                Err(err) => {
                    tracing::warn!("Could not load image: {} ({})", image_path.display(), err);
                    continue;
                }
            };

            let image = self
                .images
                .get(image_handle)
                .expect("we just loaded it successfully");

            let color_key_range = if !s.color_key_enabled.unwrap_or(false) {
                None
            } else {
                s.color_key.as_ref().map(|color_key| ColorKeyRange {
                    r: color_key.rl..=color_key.rh,
                    g: color_key.gl..=color_key.gh,
                    b: color_key.bl..=color_key.bh,
                })
            };

            let sprite = Sprite3d {
                name: s.name.clone(),
                image: image_handle,
                size: image.size,
                alpha: s.alpha,
                color_key_range,
                frames: s
                    .frames
                    .iter()
                    .map(|f| SpriteFrame {
                        top_left: glam::IVec2::new(f.x1, f.y1).as_uvec2(),
                        bottom_right: glam::IVec2::new(f.x2, f.y2).as_uvec2(),
                    })
                    .collect(),
            };

            self.sprites.insert(s.name.clone(), sprite);
        }
        */

        for s in image_defs.anim_sprite_3d.iter() {
            let image_path = PathBuf::from("textures")
                .join("object")
                .join(&s.texture_name);

            let image_handle = match self.images.load(&image_path) {
                Ok(image_handle) => image_handle,
                Err(err) => {
                    tracing::warn!("Could not load image: {} ({})", image_path.display(), err);
                    continue;
                }
            };

            let image = self
                .images
                .get(image_handle)
                .expect("we just loaded it successfully");

            let color_key_range = if !s.color_key_enabled.unwrap_or(false) {
                None
            } else {
                s.color_key.as_ref().map(|color_key| ColorKeyRange {
                    r: color_key.rl..=color_key.rh,
                    g: color_key.gl..=color_key.gh,
                    b: color_key.bl..=color_key.bh,
                })
            };

            let sprite = Sprite3d {
                name: s.name.clone(),
                image: image_handle,
                size: image.size,
                alpha: s.alpha,
                color_key_range,
                frames: s
                    .frames
                    .iter()
                    .map(|f| SpriteFrame {
                        top_left: glam::IVec2::new(f.x1, f.y1).as_uvec2(),
                        bottom_right: glam::IVec2::new(f.x2, f.y2).as_uvec2(),
                    })
                    .collect(),
            };

            self.sprites.insert(s.name.clone(), sprite);
        }
    }
}
