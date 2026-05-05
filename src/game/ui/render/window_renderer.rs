use std::sync::Arc;

use glam::{IVec2, UVec2, Vec4};

use crate::{
    engine::{
        renderer::{Gpu, RenderContext, RenderTarget, SurfaceDesc},
        shader_cache::ShaderCache,
        storage::{Handle, Storage},
    },
    game::{
        assets::{
            image::Image,
            sprites::{Sprite3d, Sprites},
        },
        render::{
            compositor::Compositor,
            geometry_buffer::GeometryBuffer,
            textures::{Texture, Textures},
        },
        ui::{Rect, u32_to_color},
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

    /// The font's default primary color (RGBA).
    #[inline]
    pub const fn primary_color(&self) -> Vec4 {
        u32_to_color(match self {
            Font::Default | Font::Small => 0xffffffff,
            Font::Clock => 0xff19ff19,
            Font::TwelvePoint | Font::FifteenPoint => 0xff4c99ff,
        })
    }

    /// The font's default secondary color (RGBA).
    #[inline]
    pub const fn secondary_color(self) -> Vec4 {
        u32_to_color(0xffffffff)
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

    /// Set a clip rect for subsequent items.
    pub fn push_clip_rect(&mut self, clip_rect: Rect) {
        self.0.push(WindowRenderItem::PushClipRect { clip_rect });
    }

    /// Clear any set clip rects.
    pub fn pop_clip_rect(&mut self) {
        self.0.push(WindowRenderItem::ClearClipRect);
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

    pub fn render_solid_rect(&mut self, rect: Rect, color: Vec4) {
        self.0.push(WindowRenderItem::SolidRect { rect, color });
    }

    /// Queues a solid-color border drawn inside the target rectangle.
    pub fn render_border(&mut self, rect: Rect, thickness: i32, color: Vec4) {
        self.0.push(WindowRenderItem::Border {
            rect,
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
    pub fn render_text(&mut self, pos: IVec2, text: &[u8], font: Font, color: Option<Vec4>) {
        self.0.push(WindowRenderItem::Text {
            pos,
            text: text.to_vec(),
            font,
            color: color.unwrap_or(font.primary_color()),
        });
    }

    /// Queues a composite of a world-view gbuffer onto the window target.
    pub fn render_world_view(&mut self, bind_group: wgpu::BindGroup) {
        self.0.push(WindowRenderItem::WorldView { bind_group });
    }
}

/// Renders all the components required for windows.
pub struct WindowRenderer {
    quad_renderer: QuadRenderer,
    textures: Arc<Textures>,
    sprites: Arc<Sprites>,
    tiled_geometries: Storage<TiledGeometry>,
    surface_size: UVec2,
    gbuffer_bind_group_layout: wgpu::BindGroupLayout,
    compositor: Compositor,
}

impl WindowRenderer {
    /// Creates the window renderer.
    pub fn new(
        gpu: Gpu,
        surface_desc: &SurfaceDesc,
        textures: Arc<Textures>,
        sprites: Arc<Sprites>,
    ) -> Self {
        let gbuffer_bind_group_layout = GeometryBuffer::create_bind_group_layout(&gpu.device);
        let mut shader_cache = ShaderCache::default();
        let compositor = Compositor::new(
            &gpu,
            surface_desc.format,
            &gbuffer_bind_group_layout,
            &mut shader_cache,
        );

        Self {
            sprites,
            textures: Arc::clone(&textures),
            quad_renderer: QuadRenderer::new(gpu, surface_desc, textures),
            tiled_geometries: Storage::default(),
            surface_size: surface_desc.size,
            gbuffer_bind_group_layout,
            compositor,
        }
    }

    /// The shared bind group layout used by all gbuffers in the world view
    /// pipeline. World renderers must build their gbuffers against this layout
    /// so the compositor can sample them.
    pub fn gbuffer_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.gbuffer_bind_group_layout
    }

    /// Create a tiled geometry render item.
    pub fn create_tiled_geometry(
        &mut self,
        image: Handle<Image>,
        dimensions: IVec2,
        chunk_dimensions: IVec2,
    ) -> Option<Handle<TiledGeometry>> {
        let render_size = self.textures.images().get(image)?.size.as_ivec2();
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
    pub fn measure_text_width(&self, text: &[u8], font: Font) -> i32 {
        let Some(handle) = self.sprites.get_handle_by_name(font.sprite_name()) else {
            return 0;
        };
        let Some(font_sprite) = self.sprites.get(handle) else {
            return 0;
        };

        let letter_spacing = font.letter_spacing();
        let mut width = 0;

        for &byte in text {
            if let Some(glyph) = font_sprite.frame(byte as usize) {
                let glyph_width = glyph.bottom_right.x - glyph.top_left.x;
                width += glyph_width + letter_spacing;
            }

            if byte == b' ' {
                width += 4;
            } else if byte == b'\t' {
                width += 12;
            }
        }

        width
    }

    /// Measures the pixel height of a text string in the given font, matching
    /// the original engine's `Calculate_Text_Height` logic. Returns the
    /// tallest glyph height found in the string.
    pub fn measure_text_height(&self, text: &[u8], font: Font) -> i32 {
        let Some(handle) = self.sprites.get_handle_by_name(font.sprite_name()) else {
            return 0;
        };
        let Some(font_sprite) = self.sprites.get(handle) else {
            return 0;
        };

        let mut height = 0;

        for &byte in text {
            if let Some(glyph) = font_sprite.frame(byte as usize) {
                let glyph_height = glyph.bottom_right.y - glyph.top_left.y;
                height = height.max(glyph_height);
            }
        }

        height
    }

    /// Returns the current render surface size in pixels.
    pub fn surface_size(&self) -> UVec2 {
        self.surface_size
    }

    /// Queues a resize for the window.
    pub fn resize(&mut self, size: UVec2) {
        self.surface_size = size;
        self.quad_renderer.resize(size);
    }

    /// Resolves window render items into quads and submits them for drawing.
    pub fn submit_render_items(
        &mut self,
        render_context: &mut RenderContext,
        render_target: &RenderTarget,
        items: &WindowRenderItems,
    ) {
        // Per-frame surface clear. Subsequent passes (quads, compositor) all
        // use LoadOp::Load and stack on top of this.
        render_context
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("window_renderer_surface_clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &render_target.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

        let mut quads = Vec::new();

        let mut clip_stack: Vec<Rect> = Vec::default();

        for item in items.0.iter() {
            match item {
                WindowRenderItem::PushClipRect { clip_rect } => {
                    // Push the intersection of the top clip rect or if the
                    // stack is empty, just the specified one.
                    let rect = clip_stack
                        .last()
                        .cloned()
                        .map(|rect| rect.intersection(*clip_rect))
                        .unwrap_or(*clip_rect);

                    clip_stack.push(rect);
                }
                WindowRenderItem::ClearClipRect => {
                    let _ = clip_stack.pop();
                }
                WindowRenderItem::Geometry => {}
                WindowRenderItem::TiledGeometry { handle, alpha } => {
                    let Some(geometry) = self.tiled_geometries.get(*handle) else {
                        continue;
                    };

                    let mut quad = Quad::texture(
                        Rect::new(IVec2::ZERO, geometry.render_size),
                        geometry.texture,
                    )
                    .with_color(Vec4::ONE.with_z(*alpha));
                    quad.clip_rect = clip_stack.last().cloned();

                    quads.push(quad);
                }
                WindowRenderItem::SolidRect { rect, color } => {
                    let mut quad = Quad::solid(*rect).with_color(*color);
                    quad.clip_rect = clip_stack.last().cloned();

                    quads.push(quad);
                }
                WindowRenderItem::Border {
                    rect,
                    thickness,
                    color,
                } => push_border_quads(
                    &mut quads,
                    *rect,
                    *thickness,
                    *color,
                    clip_stack.last().cloned(),
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

                    let color = Vec4::ONE.with_z(sprite_data.alpha.unwrap_or(*alpha));

                    let mut quad = Quad::sub_texture(
                        Rect::new(*pos, size),
                        sprite_data.texture,
                        uv_min,
                        uv_max,
                    )
                    .with_color(color);
                    quad.clip_rect = clip_stack.last().cloned();

                    quads.push(quad);
                }
                WindowRenderItem::WorldView { bind_group } => {
                    if !quads.is_empty() {
                        self.quad_renderer
                            .submit(render_context, render_target, quads.as_slice());
                        quads.clear();
                    }
                    self.compositor
                        .composite(render_context, render_target, bind_group);
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

                    for byte in text {
                        let frame_index = *byte as usize;
                        let Some(glyph_frame) = font_sprite.frame(frame_index) else {
                            continue;
                        };

                        let glyph_size = glyph_frame.bottom_right - glyph_frame.top_left;

                        // Skip zero-size glyphs (control characters, unmapped).
                        if glyph_size.x > 0 && glyph_size.y > 0 {
                            let uv_min = glyph_frame.top_left.as_vec2() / texture_size;
                            let uv_max = glyph_frame.bottom_right.as_vec2() / texture_size;

                            let color = color.with_z(color.z * alpha);

                            let mut quad = Quad::sub_texture(
                                Rect::new(IVec2::new(x, pos.y), glyph_size),
                                font_sprite.texture,
                                uv_min,
                                uv_max,
                            )
                            .with_color(color);
                            quad.clip_rect = clip_stack.last().cloned();

                            quads.push(quad);
                        }

                        x += glyph_size.x + letter_spacing;

                        // Extra spacing for space and tab, matching the original engine.
                        if *byte == b' ' {
                            x += 4;
                        } else if *byte == b'\t' {
                            x += 12;
                        }
                    }
                }
            }
        }

        self.quad_renderer
            .submit(render_context, render_target, quads.as_slice());
    }
}

#[derive(Clone)]
enum WindowRenderItem {
    Geometry,
    PushClipRect {
        clip_rect: Rect,
    },
    ClearClipRect,
    TiledGeometry {
        handle: Handle<TiledGeometry>,
        alpha: f32,
    },
    SolidRect {
        rect: Rect,
        color: Vec4,
    },
    Border {
        rect: Rect,
        thickness: i32,
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
        text: Vec<u8>,
        font: Font,
        color: Vec4,
    },
    WorldView {
        bind_group: wgpu::BindGroup,
    },
}

pub struct TiledGeometry {
    texture: Handle<Texture>,
    render_size: IVec2,
    // We store dimensions, because it came from the window base, but we don't
    // use it for rendering.
    _dimensions: IVec2,
    // Same as dimensions.
    _chunk_dimensions: IVec2,
}

fn push_border_quads(
    quads: &mut Vec<Quad>,
    rect: Rect,
    thickness: i32,
    color: Vec4,
    clip_rect: Option<Rect>,
) {
    if thickness == 0 || rect.size.x == 0 || rect.size.y == 0 {
        return;
    }

    let horizontal_thickness = thickness.min(rect.size.y);
    let vertical_thickness = thickness.min(rect.size.x);
    let inner_height = rect
        .size
        .y
        .saturating_sub(horizontal_thickness.saturating_mul(2));

    push_solid_rect(
        quads,
        Rect::new(rect.position, IVec2::new(rect.size.x, horizontal_thickness)),
        color,
        clip_rect,
    );
    push_solid_rect(
        quads,
        Rect::new(
            IVec2::new(
                rect.position.x,
                rect.position.y + rect.size.y.saturating_sub(horizontal_thickness),
            ),
            IVec2::new(rect.size.x, horizontal_thickness),
        ),
        color,
        clip_rect,
    );

    if inner_height > 0 {
        push_solid_rect(
            quads,
            Rect::new(
                IVec2::new(rect.position.x, rect.position.y + horizontal_thickness),
                IVec2::new(vertical_thickness, inner_height),
            ),
            color,
            clip_rect,
        );
        push_solid_rect(
            quads,
            Rect::new(
                IVec2::new(
                    rect.position.x + rect.size.x.saturating_sub(vertical_thickness),
                    rect.position.y + horizontal_thickness,
                ),
                IVec2::new(vertical_thickness, inner_height),
            ),
            color,
            clip_rect,
        );
    }
}

fn push_solid_rect(quads: &mut Vec<Quad>, rect: Rect, color: Vec4, clip_rect: Option<Rect>) {
    if rect.size.x == 0 || rect.size.y == 0 {
        return;
    }

    let mut quad = Quad::solid(rect).with_color(color);
    quad.clip_rect = clip_rect;

    quads.push(quad);
}
