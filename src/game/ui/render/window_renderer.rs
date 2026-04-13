use std::sync::Arc;

use glam::{IVec2, UVec2, Vec2, Vec4};

use crate::{
    engine::{
        renderer::{Frame, RenderContext, SurfaceDesc},
        storage::{Handle, Storage},
    },
    game::{
        assets::{
            asset_source::AssetSource,
            image::{BlendMode, Image},
            sprites::{Sprite3d, Sprites},
        },
        render::textures::{Texture, Textures},
    },
};

use super::quad_renderer::{Quad, QuadRenderer};

/// One of the five bitmap fonts available in the original engine.
#[derive(Clone, Copy, Debug)]
pub enum Font {
    Default,
    Small,
    Clock,
    TwelvePoint,
    FifteenPoint,
}

impl Font {
    /// The sprite name used in `image_defs.txt`.
    fn sprite_name(self) -> &'static str {
        match self {
            Font::Default => "default_font_3d",
            Font::Small => "small_font_3d",
            Font::Clock => "font_clock",
            Font::TwelvePoint => "font_12_point",
            Font::FifteenPoint => "font_15_point",
        }
    }

    /// Per-character letter spacing adjustment matching the original engine.
    fn letter_spacing(self) -> i32 {
        match self {
            Font::TwelvePoint => -2,
            _ => 0,
        }
    }

    /// The font's default primary color (RGBA) as defined in the original engine.
    pub fn default_color(self) -> Vec4 {
        match self {
            Font::Default | Font::Small => Vec4::new(1.0, 1.0, 1.0, 1.0),
            Font::Clock => Vec4::new(0.098, 1.0, 0.098, 1.0),
            Font::TwelvePoint | Font::FifteenPoint => Vec4::new(0.298, 0.6, 1.0, 1.0),
        }
    }
}

/// A list of render items that make up the window.
#[derive(Clone, Default)]
pub struct WindowRenderItems(Vec<WindowRenderItem>);

impl WindowRenderItems {
    /// Clears all queued window render items.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Appends another list of window render items, preserving their order.
    pub fn extend_from(&mut self, other: &Self) {
        self.0.extend(other.0.iter().cloned());
    }

    /// Queues a non-tiled geometry item.
    pub fn render_geometry(&mut self) {
        self.0.push(WindowRenderItem::Geometry);
    }

    /// Queues a tiled geometry item with the given alpha.
    pub fn render_tiled_geometry(&mut self, handle: Handle<TiledGeometry>, alpha: f32) {
        self.0
            .push(WindowRenderItem::TiledGeometry { handle, alpha });
    }

    /// Queues a solid-color border drawn inside the target rectangle.
    pub fn render_border(&mut self, pos: IVec2, size: UVec2, thickness: u32, color: Vec4) {
        self.0.push(WindowRenderItem::Border {
            pos,
            size,
            thickness,
            color,
        });
    }

    /// Queues a sprite item.
    pub fn render_sprite(
        &mut self,
        pos: IVec2,
        sprite: Handle<Sprite3d>,
        frame: usize,
        alpha: f32,
    ) {
        self.0.push(WindowRenderItem::Sprite {
            pos,
            sprite,
            frame,
            alpha,
        });
    }

    /// Queues a text string. Uses the font's default color unless overridden.
    pub fn render_text(&mut self, pos: IVec2, text: &str, font: Font, color: Option<Vec4>) {
        self.0.push(WindowRenderItem::Text {
            pos,
            text: text.to_owned(),
            font,
            color: color.unwrap_or(font.default_color()),
        });
    }
}

/// Renders all the components required for windows.
pub struct WindowRenderer {
    quad_renderer: QuadRenderer,
    textures: Arc<Textures>,
    sprites: Arc<Sprites>,
    tiled_geometries: Storage<TiledGeometry>,
    solid_white_texture: Handle<Texture>,
}

impl WindowRenderer {
    /// Creates the window renderer.
    pub fn new(
        render_context: RenderContext,
        surface_desc: &SurfaceDesc,
        textures: Arc<Textures>,
        sprites: Arc<Sprites>,
    ) -> Self {
        let white_image = textures.images().insert(
            "window_renderer_solid_white",
            Image::from_rgba(
                AssetSource::Generated,
                image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 255, 255, 255])),
                BlendMode::Opaque,
            ),
        );
        let solid_white_texture = textures
            .create_from_image(white_image)
            .expect("generated solid white texture should be valid");

        Self {
            sprites,
            textures: Arc::clone(&textures),
            quad_renderer: QuadRenderer::new(render_context, surface_desc, textures),
            tiled_geometries: Storage::default(),
            solid_white_texture,
        }
    }

    /// Create a tiled geometry render item.
    pub fn create_tiled_geometry(
        &mut self,
        image: Handle<Image>,
        dimensions: UVec2,
        chunk_dimensions: UVec2,
    ) -> Option<Handle<TiledGeometry>> {
        let render_size = self.textures.images().get(image)?.size;
        let texture = self.textures.create_from_image(image)?;

        Some(self.tiled_geometries.insert(TiledGeometry {
            texture,
            render_size,
            _dimensions: dimensions,
            _chunk_dimensions: chunk_dimensions,
        }))
    }

    /// Measures the pixel width of a text string in the given font, matching
    /// the original engine's `Calculate_Text_Width` logic.
    pub fn measure_text_width(&self, text: &str, font: Font) -> u32 {
        let Some(handle) = self.sprites.get_handle_by_name(font.sprite_name()) else {
            return 0;
        };
        let Some(font_sprite) = self.sprites.get(handle) else {
            return 0;
        };

        let letter_spacing = font.letter_spacing();
        let mut width = 0;

        for byte in text.bytes() {
            if let Some(glyph) = font_sprite.frame(byte as usize) {
                let glyph_width = glyph.bottom_right.x - glyph.top_left.x;
                width += (glyph_width as i32 + letter_spacing) as u32;
            }

            if byte == b' ' {
                width = (width as f32 * 4.0).round() as u32;
            } else if byte == b'\t' {
                width = (width as f32 * 12.0).round() as u32;
            }
        }

        width
    }

    /// Measures the pixel height of a text string in the given font, matching
    /// the original engine's `Calculate_Text_Height` logic. Returns the
    /// tallest glyph height found in the string.
    pub fn measure_text_height(&self, text: &str, font: Font) -> u32 {
        let Some(handle) = self.sprites.get_handle_by_name(font.sprite_name()) else {
            return 0;
        };
        let Some(font_sprite) = self.sprites.get(handle) else {
            return 0;
        };

        let mut height = 0;

        for byte in text.bytes() {
            if let Some(glyph) = font_sprite.frame(byte as usize) {
                let glyph_height = glyph.bottom_right.y - glyph.top_left.y;
                height = height.max(glyph_height);
            }
        }

        height
    }

    /// Queues a resize for the window.
    pub fn resize(&mut self, size: UVec2) {
        self.quad_renderer.resize(size);
    }

    /// Resolves window render items into quads and submits them for drawing.
    pub fn submit_render_items(&mut self, frame: &mut Frame, items: &WindowRenderItems) {
        let mut quads = Vec::new();

        for item in items.0.iter() {
            match item {
                WindowRenderItem::Geometry => {}
                WindowRenderItem::TiledGeometry { handle, alpha } => {
                    let Some(geometry) = self.tiled_geometries.get(*handle) else {
                        continue;
                    };

                    quads.push(Quad {
                        pos: Vec2::ZERO,
                        size: geometry.render_size,
                        texture: geometry.texture,
                        alpha: *alpha,
                        color: [1.0, 1.0, 1.0, 1.0],
                        uv_min: Vec2::ZERO,
                        uv_max: Vec2::ONE,
                    });
                }
                WindowRenderItem::Border {
                    pos,
                    size,
                    thickness,
                    color,
                } => push_border_quads(
                    &mut quads,
                    self.solid_white_texture,
                    *pos,
                    *size,
                    *thickness,
                    *color,
                ),
                WindowRenderItem::Sprite {
                    pos,
                    sprite,
                    frame,
                    alpha,
                } => {
                    let Some(sprite_data) = self.sprites.get(*sprite) else {
                        continue;
                    };
                    let Some(sprite_frame) = sprite_data.frame(*frame) else {
                        continue;
                    };
                    let Some(texture) = self.textures.get(sprite_data.texture) else {
                        continue;
                    };

                    let texture_size = texture.size.as_vec2();
                    let uv_min = sprite_frame.top_left.as_vec2() / texture_size;
                    let uv_max = sprite_frame.bottom_right.as_vec2() / texture_size;
                    let size = sprite_frame.bottom_right - sprite_frame.top_left;

                    quads.push(Quad {
                        pos: pos.as_vec2(),
                        size,
                        texture: sprite_data.texture,
                        alpha: sprite_data.alpha.unwrap_or(1.0) * *alpha,
                        color: [1.0, 1.0, 1.0, 1.0],
                        uv_min,
                        uv_max,
                    });
                }
                WindowRenderItem::Text {
                    pos,
                    text,
                    font,
                    color,
                } => {
                    let Some(font_sprite_handle) =
                        self.sprites.get_handle_by_name(font.sprite_name())
                    else {
                        continue;
                    };
                    let Some(font_sprite) = self.sprites.get(font_sprite_handle) else {
                        continue;
                    };
                    let Some(texture_data) = self.textures.get(font_sprite.texture) else {
                        continue;
                    };

                    let texture_size = texture_data.size.as_vec2();
                    let alpha = font_sprite.alpha.unwrap_or(1.0);
                    let letter_spacing = font.letter_spacing();
                    let mut x = pos.x;

                    for byte in text.bytes() {
                        let frame_index = byte as usize;
                        let Some(glyph_frame) = font_sprite.frame(frame_index) else {
                            continue;
                        };

                        let glyph_size = glyph_frame.bottom_right - glyph_frame.top_left;

                        // Skip zero-size glyphs (control characters, unmapped).
                        if glyph_size.x > 0 && glyph_size.y > 0 {
                            let uv_min = glyph_frame.top_left.as_vec2() / texture_size;
                            let uv_max = glyph_frame.bottom_right.as_vec2() / texture_size;

                            quads.push(Quad {
                                pos: IVec2::new(x, pos.y).as_vec2(),
                                size: glyph_size,
                                texture: font_sprite.texture,
                                alpha,
                                color: color.to_array(),
                                uv_min,
                                uv_max,
                            });
                        }

                        x += glyph_size.x as i32 + letter_spacing;

                        // Extra spacing for space and tab, matching the original engine.
                        if byte == b' ' {
                            x += 4;
                        } else if byte == b'\t' {
                            x += 12;
                        }
                    }
                }
            }
        }

        self.quad_renderer.submit(frame, quads.as_slice());
    }
}

#[derive(Clone)]
enum WindowRenderItem {
    Geometry,
    TiledGeometry {
        handle: Handle<TiledGeometry>,
        alpha: f32,
    },
    Border {
        pos: IVec2,
        size: UVec2,
        thickness: u32,
        color: Vec4,
    },
    Sprite {
        pos: IVec2,
        sprite: Handle<Sprite3d>,
        frame: usize,
        alpha: f32,
    },
    Text {
        pos: IVec2,
        text: String,
        font: Font,
        color: Vec4,
    },
}

pub struct TiledGeometry {
    texture: Handle<Texture>,
    render_size: UVec2,
    // We store dimensions, because it came from the window base, but we don't
    // use it for rendering.
    _dimensions: UVec2,
    // Same as dimensions.
    _chunk_dimensions: UVec2,
}

fn push_border_quads(
    quads: &mut Vec<Quad>,
    texture: Handle<Texture>,
    pos: IVec2,
    size: UVec2,
    thickness: u32,
    color: Vec4,
) {
    if thickness == 0 || size.x == 0 || size.y == 0 {
        return;
    }

    let horizontal_thickness = thickness.min(size.y);
    let vertical_thickness = thickness.min(size.x);
    let inner_height = size
        .y
        .saturating_sub(horizontal_thickness.saturating_mul(2));

    push_solid_rect(
        quads,
        texture,
        pos,
        UVec2::new(size.x, horizontal_thickness),
        color,
    );
    push_solid_rect(
        quads,
        texture,
        IVec2::new(
            pos.x,
            pos.y + size.y.saturating_sub(horizontal_thickness) as i32,
        ),
        UVec2::new(size.x, horizontal_thickness),
        color,
    );

    if inner_height > 0 {
        push_solid_rect(
            quads,
            texture,
            IVec2::new(pos.x, pos.y + horizontal_thickness as i32),
            UVec2::new(vertical_thickness, inner_height),
            color,
        );
        push_solid_rect(
            quads,
            texture,
            IVec2::new(
                pos.x + size.x.saturating_sub(vertical_thickness) as i32,
                pos.y + horizontal_thickness as i32,
            ),
            UVec2::new(vertical_thickness, inner_height),
            color,
        );
    }
}

fn push_solid_rect(
    quads: &mut Vec<Quad>,
    texture: Handle<Texture>,
    pos: IVec2,
    size: UVec2,
    color: Vec4,
) {
    if size.x == 0 || size.y == 0 {
        return;
    }

    quads.push(Quad {
        pos: pos.as_vec2(),
        size,
        texture,
        alpha: color.w,
        color: [color.x, color.y, color.z, 1.0],
        uv_min: Vec2::ZERO,
        uv_max: Vec2::ONE,
    });
}
