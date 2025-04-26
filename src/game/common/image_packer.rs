use glam::{UVec2, Vec2};
use image::RgbaImage;
use std::collections::HashMap;

use crate::engine::assets::Asset;

use super::image::Image;

#[derive(Debug, Clone, Copy)]
pub struct PackedRect {
    pub pos: UVec2,
    pub size: UVec2,
}

pub struct ImagePacker {
    bin_size: UVec2,
    rects: Vec<(usize, UVec2)>,
    packed: HashMap<usize, PackedRect>,
    images: HashMap<usize, Asset<Image>>,
}

impl ImagePacker {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            bin_size: UVec2::new(width, height),
            rects: Vec::new(),
            packed: HashMap::new(),
            images: HashMap::new(),
        }
    }

    pub fn add_image(&mut self, id: usize, image: Asset<Image>) -> PackedRect {
        self.rects.push((id, image.size));
        self.images.insert(id, image);

        loop {
            if let Some(packed) = Self::try_pack(self.bin_size, &self.rects) {
                self.packed = packed.clone();
                return *self.packed.get(&id).unwrap();
            } else {
                self.grow_bin();
            }
        }
    }

    pub fn bin_size(&self) -> UVec2 {
        self.bin_size
    }

    pub fn packed_rects(&self) -> &HashMap<usize, PackedRect> {
        &self.packed
    }

    pub fn build_atlas(&self) -> RgbaImage {
        let mut atlas = RgbaImage::new(self.bin_size.x, self.bin_size.y);

        for (&id, packed) in &self.packed {
            if let Some(img) = self.images.get(&id) {
                for y in 0..packed.size.y {
                    for x in 0..packed.size.x {
                        let pixel = img.data.get_pixel(x, y);
                        atlas.put_pixel(packed.pos.x + x, packed.pos.y + y, *pixel);
                    }
                }
            }
        }

        atlas
    }

    /// Returns the normalized UV offset and scale for the given image.
    pub fn uv_rect(&self, id: usize) -> Option<(Vec2, Vec2)> {
        let packed = self.packed.get(&id)?;
        let atlas_size = self.bin_size.as_vec2();
        let offset = packed.pos.as_vec2() / atlas_size;
        let scale = packed.size.as_vec2() / atlas_size;
        Some((offset, scale))
    }

    /// Adjusts a UV coordinate (from [0,1] range) to match its location in the atlas.
    pub fn adjust_uv(&self, id: usize, original_uv: Vec2) -> Option<Vec2> {
        let (offset, scale) = self.uv_rect(id)?;
        Some(offset + original_uv * scale)
    }

    fn grow_bin(&mut self) {
        if self.bin_size.x <= self.bin_size.y {
            self.bin_size.x *= 2;
        } else {
            self.bin_size.y *= 2;
        }
    }

    fn try_pack(bin_size: UVec2, rects: &[(usize, UVec2)]) -> Option<HashMap<usize, PackedRect>> {
        let mut result = HashMap::new();
        let mut cursor = UVec2::ZERO;
        let mut max_row_height = 0;

        for (id, size) in rects {
            if size.x > bin_size.x || size.y > bin_size.y {
                return None;
            }

            if cursor.x + size.x > bin_size.x {
                cursor.x = 0;
                cursor.y += max_row_height;
                max_row_height = 0;
            }

            if cursor.y + size.y > bin_size.y {
                return None;
            }

            result.insert(
                *id,
                PackedRect {
                    pos: cursor,
                    size: *size,
                },
            );

            cursor.x += size.x;
            max_row_height = max_row_height.max(size.y);
        }

        Some(result)
    }
}
