use glam::{IVec2, UVec2, Vec2, Vec4};

use crate::{
    engine::{
        renderer::{RenderContext, RenderTarget, SurfaceDesc},
        storage::Handle,
    },
    game::{
        assets::sprites::Sprite3d,
        globals,
        render::textures::Texture,
        ui::{Rect, u32_to_color, windows::window_manager::WindowManager},
    },
};

use super::ui_mesh_renderer::{UiMesh, UiMeshRenderer};

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

    /// Queues a textured rectangle drawn from a window-base geometry block.
    pub fn render_textured_rect(
        &mut self,
        rect: Rect,
        texture: Handle<Texture>,
        uv_min: Vec2,
        uv_max: Vec2,
        color: Vec4,
    ) {
        self.0.push(WindowRenderItem::TexturedRect {
            rect,
            texture,
            uv_min,
            uv_max,
            color,
        });
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

    /// Queues a prebuilt UI mesh draw.
    pub fn render_mesh(&mut self, mesh: UiMesh) {
        self.0.push(WindowRenderItem::Mesh { mesh });
    }
}

/// How the logical UI coordinate space maps to the physical surface.
///
/// - `Fixed`: 480-tall logical box, width stretches with surface aspect (min
///   640). Used in menus so the original 640×480/800×600 window bases stay
///   playable at any aspect ratio. The GPU upscales the quads to the physical
///   framebuffer via NDC mapping. When the surface aspect is narrower than
///   4:3, the UI keeps uniform vertical-driven scaling and the right edge
///   clips into the surface rather than horizontally squishing.
/// - `Native`: 1:1 with the *logical* window size (= physical / scale_factor).
///   Used in-mission so widgets land at real screen pixels at the user's
///   chosen resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiMode {
    Fixed,
    Native,
}

/// Resolves the UI logical layout size — i.e. the coordinate space windows
/// lay themselves out in. Clamped to a 640×480 minimum in `Fixed` mode so
/// widgets always have at least the original game's design real estate.
fn compute_ui_size(surface_size: UVec2, scale_factor: f32, ui_mode: UiMode) -> UVec2 {
    match ui_mode {
        UiMode::Fixed => aspect_matched(surface_size).max(UVec2::new(
            WindowRenderer::FIXED_MIN_DX,
            WindowRenderer::FIXED_DY,
        )),
        UiMode::Native => logical_surface_size(surface_size, scale_factor),
    }
}

/// Resolves the UI mesh renderer's NDC viewport — the size that maps to the full
/// framebuffer. Matches the surface aspect exactly in `Fixed` mode (no
/// minimum clamp) so the GPU stretch from UI coords to physical pixels is
/// always uniform. When `ui_size` is wider than this viewport (narrow
/// surfaces), the extra UI extent renders past the framebuffer edges and
/// gets clipped naturally.
fn compute_quad_viewport(surface_size: UVec2, scale_factor: f32, ui_mode: UiMode) -> UVec2 {
    match ui_mode {
        UiMode::Fixed => aspect_matched(surface_size),
        UiMode::Native => logical_surface_size(surface_size, scale_factor),
    }
}

/// 480-tall box whose width matches the surface aspect exactly. No floor.
fn aspect_matched(surface_size: UVec2) -> UVec2 {
    let height = WindowRenderer::FIXED_DY;
    let width = (height as f32 * surface_size.x as f32 / surface_size.y.max(1) as f32) as u32;
    UVec2::new(width.max(1), height)
}

/// Logical (DPI-independent) surface size in CSS-style pixels.
fn logical_surface_size(surface_size: UVec2, scale_factor: f32) -> UVec2 {
    let sf = scale_factor.max(f32::MIN_POSITIVE);
    (surface_size.as_vec2() / sf)
        .as_uvec2()
        .max(UVec2::splat(1))
}

/// Renders all the components required for windows.
pub struct WindowRenderer {
    ui_mesh_renderer: UiMeshRenderer,
    /// Physical framebuffer / swapchain size in pixels.
    surface_size: UVec2,
    /// Logical-to-physical pixel ratio (1.0 on most displays, 2.0 on Retina).
    scale_factor: f32,
    ui_mode: UiMode,
    /// UI coordinate-space size — what windows use for layout. In `Fixed`
    /// mode this is the aspect-matched size clamped to a 640×480 minimum,
    /// guaranteeing widgets always have at least the original game's design
    /// real estate.
    ui_size: UVec2,
    /// UI mesh renderer's NDC viewport. Matches the surface aspect exactly (no
    /// minimum clamp), so the GPU stretch is always uniform. When `ui_size`
    /// is wider than `quad_viewport` (narrow surfaces), UI extent past
    /// `quad_viewport` extends beyond the framebuffer and gets clipped.
    quad_viewport: UVec2,
}

impl WindowRenderer {
    /// `Fixed`-mode UI height. Widescreen surfaces stretch horizontally —
    /// `%screen_dx` grows past 640 while `%screen_dy` stays 480.
    const FIXED_DY: u32 = WindowManager::UI_SIZE.y as u32;
    /// `Fixed`-mode minimum width. Narrower surfaces don't squeeze below 4:3.
    const FIXED_MIN_DX: u32 = WindowManager::UI_SIZE.x as u32;

    /// Creates the window renderer in [`UiMode::Fixed`]. `GameState` flips to
    /// [`UiMode::Native`] when entering a mission.
    pub fn new(surface_desc: &SurfaceDesc) -> Self {
        let ui_mode = UiMode::Fixed;
        let ui_size = compute_ui_size(surface_desc.size, surface_desc.scale_factor, ui_mode);
        let quad_viewport =
            compute_quad_viewport(surface_desc.size, surface_desc.scale_factor, ui_mode);

        let viewport_surface = SurfaceDesc {
            size: quad_viewport,
            format: surface_desc.format,
            scale_factor: surface_desc.scale_factor,
        };

        Self {
            ui_mesh_renderer: UiMeshRenderer::new(&viewport_surface),
            surface_size: surface_desc.size,
            scale_factor: surface_desc.scale_factor,
            ui_mode,
            ui_size,
            quad_viewport,
        }
    }

    /// Updates the physical surface size + scale factor, then recomputes
    /// `ui_size` and reconfigures the UI mesh renderer viewport.
    pub fn resize(&mut self, surface_size: UVec2, scale_factor: f32) {
        self.surface_size = surface_size;
        self.scale_factor = scale_factor;
        self.refresh_layout();
    }

    /// Switches the UI mode. Returns the new UI size if it changed (so the
    /// caller can re-layout windows); returns `None` otherwise.
    pub fn set_ui_mode(&mut self, ui_mode: UiMode) -> Option<UVec2> {
        if self.ui_mode == ui_mode {
            return None;
        }
        self.ui_mode = ui_mode;
        let ui_changed = self.refresh_layout();
        ui_changed.then_some(self.ui_size)
    }

    /// Recomputes `ui_size` and `quad_viewport` from the current surface +
    /// scale + mode, pushing any viewport change to the quad renderer.
    /// Returns whether `ui_size` actually moved (i.e. whether layouts need
    /// to re-resolve).
    fn refresh_layout(&mut self) -> bool {
        let new_ui_size = compute_ui_size(self.surface_size, self.scale_factor, self.ui_mode);
        let new_viewport =
            compute_quad_viewport(self.surface_size, self.scale_factor, self.ui_mode);

        if new_viewport != self.quad_viewport {
            self.quad_viewport = new_viewport;
            self.ui_mesh_renderer.resize(new_viewport);
        }

        if new_ui_size == self.ui_size {
            return false;
        }
        self.ui_size = new_ui_size;
        true
    }

    /// Returns the current physical framebuffer size in pixels.
    pub fn surface_size(&self) -> UVec2 {
        self.surface_size
    }

    /// Returns the current UI logical size. This is the coordinate space
    /// windows lay out in and what `%screen_dx`/`%screen_dy` resolve to.
    /// Differs from the physical surface in [`UiMode::Fixed`] and on high-DPI
    /// displays in [`UiMode::Native`].
    pub fn ui_size(&self) -> UVec2 {
        self.ui_size
    }

    /// Returns the current UI mode.
    pub fn ui_mode(&self) -> UiMode {
        self.ui_mode
    }

    /// Returns the current scale factor (logical-to-physical pixel ratio).
    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    /// Maps a physical-pixel position (as delivered by winit) into UI
    /// coordinates. The mapping uses the UI renderer viewport rather than
    /// `ui_size`, so the result lines up with what the user sees rendered —
    /// in narrow surfaces (where `ui_size` extends past the viewport) the
    /// reachable UI x-range stops at `quad_viewport.x`.
    pub fn physical_to_ui_position(&self, position: IVec2) -> IVec2 {
        let surface = self.surface_size.as_ivec2();
        if surface.x <= 0 || surface.y <= 0 {
            return position;
        }
        let viewport = self.quad_viewport.as_ivec2();
        IVec2::new(
            (position.x as i64 * viewport.x as i64 / surface.x as i64) as i32,
            (position.y as i64 * viewport.y as i64 / surface.y as i64) as i32,
        )
    }

    /// Measures the pixel width of a text string in the given font, matching
    /// the original engine's `Calculate_Text_Width` logic.
    pub fn measure_text_width(&self, text: &[u8], font: Font) -> i32 {
        let Some(handle) = globals::sprites().get_handle_by_name(font.sprite_name()) else {
            return 0;
        };
        let Some(font_sprite) = globals::sprites().get(handle) else {
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
        let Some(handle) = globals::sprites().get_handle_by_name(font.sprite_name()) else {
            return 0;
        };
        let Some(font_sprite) = globals::sprites().get(handle) else {
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

    /// Resolves window render items into UI meshes and submits them for drawing.
    pub fn submit_render_items(
        &mut self,
        render_context: &mut RenderContext,
        render_target: &RenderTarget,
        items: &WindowRenderItems,
    ) {
        let mut meshes = Vec::new();
        let white_texture = self.ui_mesh_renderer.solid_white_texture();

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
                WindowRenderItem::TexturedRect {
                    rect,
                    texture,
                    uv_min,
                    uv_max,
                    color,
                } => {
                    let mesh = with_clip_rect(
                        UiMesh::textured_rect(*rect, *texture, *uv_min, *uv_max, *color),
                        clip_stack.last().cloned(),
                    );
                    meshes.push(mesh);
                }
                WindowRenderItem::SolidRect { rect, color } => {
                    let mesh = with_clip_rect(
                        UiMesh::textured_rect(*rect, white_texture, Vec2::ZERO, Vec2::ONE, *color),
                        clip_stack.last().cloned(),
                    );

                    meshes.push(mesh);
                }
                WindowRenderItem::Border {
                    rect,
                    thickness,
                    color,
                } => push_border_meshes(
                    &mut meshes,
                    white_texture,
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
                    let Some(sprite_data) = globals::sprites().get(*sprite) else {
                        continue;
                    };
                    let Some(sprite_frame) = sprite_data.frame(*frame) else {
                        continue;
                    };
                    let Some(texture) = globals::textures().get(sprite_data.texture) else {
                        continue;
                    };

                    let texture_size = texture.size.as_vec2();
                    let uv_min = sprite_frame.top_left.as_vec2() / texture_size;
                    let uv_max = sprite_frame.bottom_right.as_vec2() / texture_size;
                    let size = sprite_frame.bottom_right - sprite_frame.top_left;

                    let color = Vec4::ONE.with_w(sprite_data.alpha.unwrap_or(*alpha));

                    let mesh = with_clip_rect(
                        UiMesh::textured_rect(
                            Rect::new(*pos, size),
                            sprite_data.texture,
                            uv_min,
                            uv_max,
                            color,
                        ),
                        clip_stack.last().cloned(),
                    );

                    meshes.push(mesh);
                }
                WindowRenderItem::Text {
                    pos,
                    text,
                    font,
                    color,
                } => {
                    let Some(font_sprite_handle) =
                        globals::sprites().get_handle_by_name(font.sprite_name())
                    else {
                        continue;
                    };
                    let Some(font_sprite) = globals::sprites().get(font_sprite_handle) else {
                        continue;
                    };
                    let Some(texture_data) = globals::textures().get(font_sprite.texture) else {
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

                            let color = Vec4::new(color.x, color.y, color.z, color.w * alpha);

                            let mesh = with_clip_rect(
                                UiMesh::textured_rect(
                                    Rect::new(IVec2::new(x, pos.y), glyph_size),
                                    font_sprite.texture,
                                    uv_min,
                                    uv_max,
                                    color,
                                ),
                                clip_stack.last().cloned(),
                            );

                            meshes.push(mesh);
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
                WindowRenderItem::Mesh { mesh } => {
                    meshes.push(with_clip_rect(mesh.clone(), clip_stack.last().cloned()));
                }
            }
        }

        self.ui_mesh_renderer
            .submit(render_context, render_target, meshes.as_slice());
    }
}

#[derive(Clone)]
enum WindowRenderItem {
    PushClipRect {
        clip_rect: Rect,
    },
    ClearClipRect,
    TexturedRect {
        rect: Rect,
        texture: Handle<Texture>,
        uv_min: Vec2,
        uv_max: Vec2,
        color: Vec4,
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
    Mesh {
        mesh: UiMesh,
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

fn push_border_meshes(
    meshes: &mut Vec<UiMesh>,
    white_texture: Handle<Texture>,
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
        meshes,
        white_texture,
        Rect::new(rect.position, IVec2::new(rect.size.x, horizontal_thickness)),
        color,
        clip_rect,
    );
    push_solid_rect(
        meshes,
        white_texture,
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
            meshes,
            white_texture,
            Rect::new(
                IVec2::new(rect.position.x, rect.position.y + horizontal_thickness),
                IVec2::new(vertical_thickness, inner_height),
            ),
            color,
            clip_rect,
        );
        push_solid_rect(
            meshes,
            white_texture,
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

fn push_solid_rect(
    meshes: &mut Vec<UiMesh>,
    white_texture: Handle<Texture>,
    rect: Rect,
    color: Vec4,
    clip_rect: Option<Rect>,
) {
    if rect.size.x == 0 || rect.size.y == 0 {
        return;
    }

    meshes.push(with_clip_rect(
        UiMesh::textured_rect(rect, white_texture, Vec2::ZERO, Vec2::ONE, color),
        clip_rect,
    ));
}

fn with_clip_rect(mut mesh: UiMesh, clip_rect: Option<Rect>) -> UiMesh {
    mesh.clip_rect = clip_rect;
    mesh
}
