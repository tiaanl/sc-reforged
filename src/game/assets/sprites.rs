use std::{ops::RangeInclusive, path::PathBuf};

use glam::IVec2;

use crate::{
    engine::{
        assets::AssetError,
        storage::{Handle, StorageMap},
    },
    game::{
        config::{ImageDefs, load_config},
        globals,
        render::textures::Texture,
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
    pub top_left: IVec2,
    pub bottom_right: IVec2,
}

#[derive(Debug)]
pub struct Sprite3d {
    pub name: String,
    pub texture: Handle<Texture>,
    pub size: glam::UVec2,
    pub alpha: Option<f32>,
    pub color_key_range: Option<ColorKeyRange>,
    pub frames: Vec<SpriteFrame>,
}

impl Sprite3d {
    /// Returns the frame at the given index, if it exists.
    pub fn frame(&self, index: usize) -> Option<&SpriteFrame> {
        self.frames.get(index)
    }
}

pub struct Sprites {
    sprites: StorageMap<String, Sprite3d>,
}

impl Sprites {
    pub fn new() -> Result<Self, AssetError> {
        let mut sprites = Self {
            sprites: StorageMap::default(),
        };

        let image_defs: ImageDefs = load_config(PathBuf::from("config").join("image_defs.txt"))?;
        sprites.load_image_defs(&image_defs);

        Ok(sprites)
    }

    #[inline]
    /// Returns a sprite by handle.
    pub fn get(&self, handle: Handle<Sprite3d>) -> Option<&Sprite3d> {
        self.sprites.get(handle)
    }

    /// Returns a sprite handle by its configured name.
    pub fn get_handle_by_name(&self, name: &str) -> Option<Handle<Sprite3d>> {
        self.sprites.get_handle_by_key(&String::from(name))
    }

    /// Returns a sprite by its configured name.
    pub fn get_by_name(&self, name: &str) -> Option<&Sprite3d> {
        self.sprites.get_by_key(&String::from(name))
    }

    /// Loads sprite definitions from `image_defs.txt`.
    fn load_image_defs(&mut self, image_defs: &ImageDefs) {
        for s in image_defs.sprite_3d.iter() {
            self.insert_sprite(
                &s.name,
                &s.texture_name,
                s.alpha,
                s.color_key_enabled.unwrap_or(false),
                s.color_key.as_ref(),
                &s.frames,
            );
        }

        for s in image_defs.anim_sprite_3d.iter() {
            self.insert_sprite(
                &s.name,
                &s.texture_name,
                s.alpha,
                s.color_key_enabled.unwrap_or(false),
                s.color_key.as_ref(),
                &s.frames,
            );
        }
    }

    fn insert_sprite(
        &mut self,
        name: &str,
        texture_name: &str,
        alpha: Option<f32>,
        color_key_enabled: bool,
        color_key: Option<&crate::game::config::ColorKey>,
        frames: &[crate::game::config::SpriteFrame],
    ) {
        let image_path = PathBuf::from("textures").join("object").join(texture_name);

        let image_handle = match globals::images().load(&image_path) {
            Ok(image_handle) => image_handle,
            Err(err) => {
                tracing::warn!("Could not load image: {} ({})", image_path.display(), err);
                return;
            }
        };

        let texture_handle = match globals::textures().create_from_image(image_handle) {
            Some(handle) => handle,
            None => {
                tracing::warn!(
                    "Could not create texture from image: {}",
                    image_path.display(),
                );
                return;
            }
        };

        let Some(size) = globals::textures().size(texture_handle) else {
            tracing::warn!("Invalid image size: {}", image_path.display());
            return;
        };

        let color_key_range = if !color_key_enabled {
            None
        } else {
            color_key.map(|color_key| ColorKeyRange {
                r: color_key.rl..=color_key.rh,
                g: color_key.gl..=color_key.gh,
                b: color_key.bl..=color_key.bh,
            })
        };

        let sprite = Sprite3d {
            name: name.to_owned(),
            texture: texture_handle,
            size,
            alpha,
            color_key_range,
            frames: frames
                .iter()
                .map(|f| SpriteFrame {
                    top_left: IVec2::new(f.x1, f.y1),
                    bottom_right: IVec2::new(f.x2, f.y2),
                })
                .collect(),
        };

        self.sprites.insert(name.to_owned(), sprite);
    }
}
