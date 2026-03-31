use std::sync::Arc;

use glam::{UVec2, Vec2, Vec4};

use crate::{
    engine::{
        renderer::{Frame, RenderContext, SurfaceDesc},
        storage::{Handle, Storage},
    },
    game::{
        assets::{
            image::Image,
            sprites::{Sprite3d, Sprites},
        },
        render::textures::{Texture, Textures},
        scenes::main_menu::quad_renderer::{Quad, QuadRenderer},
    },
};

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
    fn letter_spacing(self) -> f32 {
        match self {
            Font::TwelvePoint => -2.0,
            _ => 0.0,
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
#[derive(Default)]
pub struct RenderItems(Vec<RenderItem>);

impl RenderItems {
    /// Clears all queued window render items.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Queues a non-tiled geometry item.
    pub fn render_geometry(&mut self) {
        self.0.push(RenderItem::Geometry);
    }

    /// Queues a tiled geometry item with the given alpha.
    pub fn render_tiled_geometry(&mut self, handle: Handle<TiledGeometry>, alpha: f32) {
        self.0.push(RenderItem::TiledGeometry { handle, alpha });
    }

    /// Queues a sprite item.
    pub fn render_sprite(&mut self, pos: Vec2, sprite: Handle<Sprite3d>, frame: usize, alpha: f32) {
        self.0.push(RenderItem::Sprite {
            pos,
            sprite,
            frame,
            alpha,
        });
    }

    /// Queues a text string. Uses the font's default color unless overridden.
    pub fn render_text(&mut self, pos: Vec2, text: &str, font: Font, color: Option<Vec4>) {
        self.0.push(RenderItem::Text {
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
}

impl WindowRenderer {
    /// Creates the window renderer.
    pub fn new(
        render_context: RenderContext,
        surface_desc: &SurfaceDesc,
        textures: Arc<Textures>,
        sprites: Arc<Sprites>,
    ) -> Self {
        Self {
            sprites,
            textures: Arc::clone(&textures),
            quad_renderer: QuadRenderer::new(render_context, surface_desc, textures),
            tiled_geometries: Storage::default(),
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
    pub fn measure_text_width(&self, text: &str, font: Font) -> f32 {
        let Some(handle) = self.sprites.get_handle_by_name(font.sprite_name()) else {
            return 0.0;
        };
        let Some(font_sprite) = self.sprites.get(handle) else {
            return 0.0;
        };

        let letter_spacing = font.letter_spacing();
        let mut width = 0.0_f32;

        for byte in text.bytes() {
            if let Some(glyph) = font_sprite.frame(byte as usize) {
                let glyph_width = glyph.bottom_right.x as f32 - glyph.top_left.x as f32;
                width += glyph_width + letter_spacing;
            }

            if byte == b' ' {
                width += 4.0;
            } else if byte == b'\t' {
                width += 12.0;
            }
        }

        width
    }

    /// Measures the pixel height of a text string in the given font, matching
    /// the original engine's `Calculate_Text_Height` logic. Returns the
    /// tallest glyph height found in the string.
    pub fn measure_text_height(&self, text: &str, font: Font) -> f32 {
        let Some(handle) = self.sprites.get_handle_by_name(font.sprite_name()) else {
            return 0.0;
        };
        let Some(font_sprite) = self.sprites.get(handle) else {
            return 0.0;
        };

        let mut height = 0.0_f32;

        for byte in text.bytes() {
            if let Some(glyph) = font_sprite.frame(byte as usize) {
                let glyph_height = glyph.bottom_right.y as f32 - glyph.top_left.y as f32;
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
    pub fn submit_render_items(&mut self, frame: &mut Frame, items: &RenderItems) {
        let mut quads = Vec::new();

        for item in items.0.iter() {
            match item {
                RenderItem::Geometry => {}
                RenderItem::TiledGeometry { handle, alpha } => {
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
                RenderItem::Sprite {
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
                    let Some(texture) = self.textures.create_from_image(sprite_data.image) else {
                        continue;
                    };
                    let Some(texture_data) = self.textures.get(texture) else {
                        continue;
                    };

                    let texture_size = texture_data.size.as_vec2();
                    let uv_min = sprite_frame.top_left.as_vec2() / texture_size;
                    let uv_max = sprite_frame.bottom_right.as_vec2() / texture_size;
                    let size = sprite_frame.bottom_right - sprite_frame.top_left;

                    quads.push(Quad {
                        pos: *pos,
                        size,
                        texture,
                        alpha: sprite_data.alpha.unwrap_or(1.0) * *alpha,
                        color: [1.0, 1.0, 1.0, 1.0],
                        uv_min,
                        uv_max,
                    });
                }
                RenderItem::Text {
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
                    let Some(texture) = self.textures.create_from_image(font_sprite.image) else {
                        continue;
                    };
                    let Some(texture_data) = self.textures.get(texture) else {
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
                                pos: Vec2::new(x, pos.y),
                                size: glyph_size,
                                texture,
                                alpha,
                                color: color.to_array(),
                                uv_min,
                                uv_max,
                            });
                        }

                        x += glyph_size.x as f32 + letter_spacing;

                        // Extra spacing for space and tab, matching the original engine.
                        if byte == b' ' {
                            x += 4.0;
                        } else if byte == b'\t' {
                            x += 12.0;
                        }
                    }
                }
            }
        }

        self.quad_renderer.submit(frame, quads.as_slice());
    }
}

enum RenderItem {
    Geometry,
    TiledGeometry {
        handle: Handle<TiledGeometry>,
        alpha: f32,
    },
    Sprite {
        pos: Vec2,
        sprite: Handle<Sprite3d>,
        frame: usize,
        alpha: f32,
    },
    Text {
        pos: Vec2,
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
