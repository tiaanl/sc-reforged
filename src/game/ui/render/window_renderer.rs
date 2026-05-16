use glam::{IVec2, UVec2, Vec2, Vec4};

use crate::{
    engine::{
        renderer::{RenderContext, RenderTarget, SurfaceDesc},
        storage::{Handle, Storage},
    },
    game::{
        assets::{image::Image, sprites::Sprite3d},
        globals,
        render::textures::Texture,
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
}

/// How the UI's logical coordinate space relates to the physical surface.
///
/// - `Logical`: a 480-tall logical box stretched horizontally to the surface
///   aspect — used outside gameplay (menus, briefing, etc.). Lets the original
///   640x480/800x600 window bases reflow cleanly to widescreen.
/// - `Native`: 1:1 with the physical surface — used in-game so widgets land at
///   real screen pixels and `%screen_dx` / `%screen_dy` reflect the actual
///   window dimensions, matching the original game's behavior at the active
///   display resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiScale {
    Logical,
    Native,
}

/// Renders all the components required for windows.
pub struct WindowRenderer {
    quad_renderer: QuadRenderer,
    tiled_geometries: Storage<TiledGeometry>,
    surface_size: UVec2,
    logical_size: UVec2,
    surface_format: wgpu::TextureFormat,
    ui_scale: UiScale,
    ui_target: UiRenderTarget,
    ui_presenter: UiPresenter,
}

impl WindowRenderer {
    /// The vertical logical UI resolution (locked). Widescreen surfaces stretch
    /// horizontally — `%screen_dx` grows past 640 while `%screen_dy` stays 480.
    pub const LOGICAL_DY: u32 = 480;
    /// Minimum logical width — surfaces narrower than 4:3 letterbox instead of
    /// squeezing the bar below its base width.
    pub const MIN_LOGICAL_DX: u32 = 640;
    /// The base (un-stretched) logical UI size — handy as a default.
    pub const LOGICAL_SIZE: UVec2 = UVec2::new(Self::MIN_LOGICAL_DX, Self::LOGICAL_DY);

    /// Resolves a (physical, scale) pair to a logical UI size. Native is 1:1;
    /// Logical locks height to [`Self::LOGICAL_DY`] and stretches width with
    /// aspect, divided by the integer scale that fits both
    /// [`Self::MIN_LOGICAL_DX`] × [`Self::LOGICAL_DY`] into the surface so the
    /// presentation rect can never exceed the surface bounds.
    pub fn compute_logical_size(physical: UVec2, ui_scale: UiScale) -> UVec2 {
        match ui_scale {
            UiScale::Native => physical.max(UVec2::splat(1)),
            UiScale::Logical => {
                let scale = Self::compute_logical_scale(physical);
                let dx = (physical.x / scale).max(Self::MIN_LOGICAL_DX);
                UVec2::new(dx, Self::LOGICAL_DY)
            }
        }
    }

    fn compute_logical_scale(physical: UVec2) -> u32 {
        let vertical = physical.y / Self::LOGICAL_DY;
        let horizontal = physical.x / Self::MIN_LOGICAL_DX;
        vertical.min(horizontal).max(1)
    }

    /// Creates the window renderer. Starts in [`UiScale::Logical`]; the game
    /// state flips to [`UiScale::Native`] when entering gameplay.
    pub fn new(surface_desc: &SurfaceDesc) -> Self {
        let ui_scale = UiScale::Logical;
        let logical_size = Self::compute_logical_size(surface_desc.size, ui_scale);
        let logical_surface = SurfaceDesc {
            size: logical_size,
            format: surface_desc.format,
        };
        let ui_presenter = UiPresenter::new(surface_desc.format);
        let ui_target = UiRenderTarget::new(
            logical_size,
            surface_desc.format,
            ui_presenter.bind_group_layout(),
            ui_presenter.sampler(),
        );

        Self {
            quad_renderer: QuadRenderer::new(&logical_surface),
            tiled_geometries: Storage::default(),
            surface_size: surface_desc.size,
            logical_size,
            surface_format: surface_desc.format,
            ui_scale,
            ui_target,
            ui_presenter,
        }
    }

    /// Switches the UI between the stretched 480-tall logical box and a 1:1
    /// native mapping to the physical surface. Returns the new logical size if
    /// it changed (so the caller can re-layout windows); returns `None` if the
    /// mode was already active.
    pub fn set_ui_scale(&mut self, ui_scale: UiScale) -> Option<UVec2> {
        if self.ui_scale == ui_scale {
            return None;
        }
        self.ui_scale = ui_scale;
        self.refresh_logical_size().then_some(self.logical_size)
    }

    pub fn ui_scale(&self) -> UiScale {
        self.ui_scale
    }

    /// Recomputes the logical size from the current surface size + ui_scale,
    /// rebuilding the offscreen target and quad viewport if it changed.
    /// Returns whether the size actually moved.
    fn refresh_logical_size(&mut self) -> bool {
        let new_logical = Self::compute_logical_size(self.surface_size, self.ui_scale);
        if new_logical == self.logical_size {
            return false;
        }
        self.logical_size = new_logical;
        self.ui_target = UiRenderTarget::new(
            new_logical,
            self.surface_format,
            self.ui_presenter.bind_group_layout(),
            self.ui_presenter.sampler(),
        );
        self.quad_renderer.resize(new_logical);
        true
    }

    /// Create a tiled geometry render item.
    pub fn create_tiled_geometry(
        &mut self,
        image: Handle<Image>,
        dimensions: IVec2,
        chunk_dimensions: IVec2,
    ) -> Option<Handle<TiledGeometry>> {
        let render_size = globals::images().get(image)?.size.as_ivec2();
        let texture = globals::textures().create_from_image(image)?;

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

    /// Returns the current logical UI size — locked to 480 in height, stretched
    /// horizontally to match the surface aspect.
    pub fn surface_size(&self) -> UVec2 {
        self.logical_size
    }

    /// Returns the physical swapchain surface size in pixels.
    pub fn physical_surface_size(&self) -> UVec2 {
        self.surface_size
    }

    /// Returns the largest integer UI scale that fits the current surface. In
    /// [`UiScale::Native`] this is always 1 (no scaling). In Logical it's
    /// constrained by both axes so the presentation rect never exceeds the
    /// surface bounds.
    pub fn integer_scale(&self) -> u32 {
        match self.ui_scale {
            UiScale::Native => 1,
            UiScale::Logical => Self::compute_logical_scale(self.surface_size),
        }
    }

    /// Returns the centered physical rect used to present the logical UI. In
    /// Native mode this is the full surface; in Logical mode it's a centered
    /// integer-scaled rect with letterboxing on whichever axis has slack.
    pub fn presentation_rect(&self) -> Rect {
        let scale = self.integer_scale();
        let size = (self.logical_size * scale).as_ivec2();
        if size.x > self.surface_size.x as i32 || size.y > self.surface_size.y as i32 {
            return Rect::default();
        }
        let position = (self.surface_size.as_ivec2() - size) / 2;

        Rect::new(position, size)
    }

    /// Converts a physical surface position to logical UI coordinates.
    pub fn surface_to_logical_position(&self, position: IVec2) -> Option<IVec2> {
        let scale = self.integer_scale() as i32;
        if scale == 0 {
            return None;
        }

        let presentation_rect = self.presentation_rect();
        presentation_rect
            .contains(position)
            .then_some((position - presentation_rect.position) / scale)
    }

    /// Queues a resize for the window. If the derived logical size changes,
    /// rebuilds the offscreen UI target and reconfigures the quad renderer's
    /// viewport so widgets keep mapping 1:1 into the target.
    pub fn resize(&mut self, size: UVec2) {
        self.surface_size = size;
        self.refresh_logical_size();
    }

    /// Resolves window render items into quads and submits them for drawing.
    pub fn submit_render_items(
        &mut self,
        render_context: &mut RenderContext,
        render_target: &RenderTarget,
        items: &WindowRenderItems,
    ) {
        self.ui_target.clear(render_context);
        let ui_render_target = self.ui_target.render_target();

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
                WindowRenderItem::TexturedRect {
                    rect,
                    texture,
                    uv_min,
                    uv_max,
                    color,
                } => {
                    let mut quad =
                        Quad::sub_texture(*rect, *texture, *uv_min, *uv_max).with_color(*color);
                    quad.clip_rect = clip_stack.last().cloned();
                    quads.push(quad);
                }
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
            .submit(render_context, &ui_render_target, quads.as_slice());

        self.ui_presenter.present(
            render_context,
            render_target,
            self.ui_target.bind_group(),
            self.presentation_rect(),
        );
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
}

struct UiRenderTarget {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    size: UVec2,
}

impl UiRenderTarget {
    /// Creates a transparent offscreen target for logical UI rendering.
    fn new(
        size: UVec2,
        format: wgpu::TextureFormat,
        bind_group_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
    ) -> Self {
        let texture = globals::gpu()
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("ui_logical_render_target"),
                size: wgpu::Extent3d {
                    width: size.x.max(1),
                    height: size.y.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = globals::gpu()
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ui_logical_present_bind_group"),
                layout: bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                ],
            });

        Self {
            _texture: texture,
            view,
            bind_group,
            size,
        }
    }

    /// Clears the logical UI target to transparent black.
    fn clear(&self, render_context: &mut RenderContext) {
        render_context
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui_logical_target_clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
    }

    /// Returns a render target view for drawing logical UI quads.
    fn render_target(&self) -> RenderTarget {
        RenderTarget {
            view: self.view.clone(),
            size: self.size,
        }
    }

    /// Returns the bind group used by the UI presenter.
    fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

struct UiPresenter {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl UiPresenter {
    /// Creates the pipeline used to present logical UI to the surface.
    fn new(target_format: wgpu::TextureFormat) -> Self {
        let device = &globals::gpu().device;
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ui_presenter_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ui_presenter_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ui_presenter_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "present.wgsl"
            ))),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ui_presenter_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui_presenter_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            bind_group_layout,
            sampler,
        }
    }

    /// Returns the bind group layout used by UI source textures.
    fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Returns the nearest-neighbor sampler used by the presenter.
    fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    /// Presents the logical UI texture to a physical surface rect.
    fn present(
        &self,
        render_context: &mut RenderContext,
        render_target: &RenderTarget,
        bind_group: &wgpu::BindGroup,
        rect: Rect,
    ) {
        if rect.size.x <= 0 || rect.size.y <= 0 {
            return;
        }

        let mut render_pass =
            render_context
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("ui_presenter_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &render_target.view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.set_viewport(
            rect.position.x as f32,
            rect.position.y as f32,
            rect.size.x as f32,
            rect.size.y as f32,
            0.0,
            1.0,
        );
        render_pass.set_scissor_rect(
            rect.position.x as u32,
            rect.position.y as u32,
            rect.size.x as u32,
            rect.size.y as u32,
        );
        render_pass.draw(0..3, 0..1);
    }
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
