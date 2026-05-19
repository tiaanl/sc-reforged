use std::{path::PathBuf, sync::Arc};

use glam::{IVec2, Vec2, Vec4};

use crate::{
    engine::storage::Handle,
    game::{
        config::windows::{
            Geometry, GeometryNormal, GeometryTiled, Vertex, WindowBase, WindowBaseLayout,
            WindowCtx,
        },
        globals,
        render::textures::Texture,
        ui::{Rect, render::window_renderer::WindowRenderItems},
    },
};

/// Background art for a window, driven by a `WindowBase` config.
///
/// A window base declares a list of geometries — `Normal` (textured 4-vertex
/// quads) and `Tiled` (large background images, originally chunked for
/// streaming but drawn here as a single textured quad) — that together form
/// the window's visual chrome. The bottombar, command pad, inventory bar
/// etc. all get their look from this.
///
/// Defaults to empty; windows without a backing window base just leave it
/// unset.
#[derive(Default)]
pub struct Geometries {
    /// The source window base. `None` when no base is bound.
    window_base: Option<Arc<WindowBase>>,
    /// Layout resolved against the most recent UI size.
    layout: WindowBaseLayout,
    /// Per-geometry texture handle. `None` means a load failure.
    textures: Vec<Option<Handle<Texture>>>,
}

impl Geometries {
    /// Builds a geometry list from a window base, resolved for the given UI
    /// size. Loads all referenced textures up front.
    pub fn from_window_base(window_base: Arc<WindowBase>, ui_size: IVec2) -> Self {
        let mut geometries = Self {
            window_base: Some(window_base),
            layout: WindowBaseLayout::default(),
            textures: Vec::new(),
        };
        geometries.resolve(ui_size);
        geometries
    }

    /// Returns the most recently resolved layout — used by windows that size
    /// or position themselves from window-base metadata.
    pub fn layout(&self) -> &WindowBaseLayout {
        &self.layout
    }

    /// Re-resolves the layout against a new UI size and reloads geometry
    /// textures. No-op if no window base is bound.
    pub fn on_resize(&mut self, ui_size: IVec2) {
        self.resolve(ui_size);
    }

    fn resolve(&mut self, ui_size: IVec2) {
        let Some(window_base) = &self.window_base else {
            return;
        };
        self.layout = window_base.layout(&WindowCtx::from_logical_size(ui_size));
        self.textures = self
            .layout
            .geometries
            .iter()
            .map(|geometry| match geometry {
                Geometry::Normal(normal) => load_texture(&normal.texture),
                // Tiled `jpg_name` is bare (no extension); the file lives at
                // `textures/interface/<name>.jpg`.
                Geometry::Tiled(tiled) => load_texture(&format!("{}.jpg", tiled.jpg_name)),
            })
            .collect();
    }

    /// Emits render items for every geometry, offset by `origin` (the parent
    /// window's position in UI coordinates).
    pub fn render(&self, origin: IVec2, render_items: &mut WindowRenderItems) {
        for (geometry, texture) in self.layout.geometries.iter().zip(self.textures.iter()) {
            let Some(texture) = *texture else {
                continue;
            };

            let (rect, uv_min, uv_max, color) = match geometry {
                Geometry::Normal(normal) => {
                    let Some(quad) = compute_normal_quad(normal) else {
                        continue;
                    };
                    quad
                }
                Geometry::Tiled(tiled) => compute_tiled_quad(tiled),
            };

            render_items.render_textured_rect(
                rect.offset(origin),
                texture,
                uv_min,
                uv_max,
                color,
            );
        }
    }
}

/// Resolves a window-base geometry texture name (e.g. `simu1_ck.bmp`) to a
/// loaded texture handle. Today we only look in `textures/interface/`, which
/// covers every texture referenced by the bundled window bases.
fn load_texture(name: &str) -> Option<Handle<Texture>> {
    let path = PathBuf::from("textures").join("interface").join(name);
    let image = match globals::images().load(&path) {
        Ok(handle) => handle,
        Err(err) => {
            tracing::warn!("Failed to load window-base texture {name}: {err}");
            return None;
        }
    };
    globals::textures().create_from_image(image)
}

/// Approximates a 4-vertex window-base geometry as an axis-aligned textured
/// rectangle. Returns `None` if the block has fewer than four vertices.
///
/// TODO: This drops per-vertex color gradients and ignores the `AUTO` UV mode.
/// To match the original we'd need a triangle-soup renderer with per-vertex
/// color and UV interpolation.
fn compute_normal_quad(geometry: &GeometryNormal) -> Option<(Rect, Vec2, Vec2, Vec4)> {
    if geometry.vertices.len() < 4 {
        return None;
    }

    let (min, max) = bounding_box(&geometry.vertices);
    let rect = Rect::new(min, max - min);

    let v0 = geometry.vertices[0];
    let color = Vec4::new(v0.r, v0.g, v0.b, v0.a);

    let pack_dx = geometry.texture_pack_dx.max(1) as f32;
    let pack_dy = geometry.texture_pack_dy.max(1) as f32;

    let uv0 = uv_or_zero(geometry.vertices[0], pack_dx, pack_dy);
    let uv2 = uv_or_zero(geometry.vertices[2], pack_dx, pack_dy);
    let uv_min = uv0.min(uv2);
    let uv_max = uv0.max(uv2);

    Some((rect, uv_min, uv_max, color))
}

/// Renders a tiled background as a single full-texture quad at the window
/// origin. The chunking metadata is ignored — the original engine used it for
/// streaming large jpgs in chunks, but it's not needed here.
fn compute_tiled_quad(geometry: &GeometryTiled) -> (Rect, Vec2, Vec2, Vec4) {
    let size = IVec2::new(geometry.dimensions[0], geometry.dimensions[1]);
    (Rect::new(IVec2::ZERO, size), Vec2::ZERO, Vec2::ONE, Vec4::ONE)
}

fn bounding_box(vertices: &[Vertex]) -> (IVec2, IVec2) {
    let mut min = IVec2::new(i32::MAX, i32::MAX);
    let mut max = IVec2::new(i32::MIN, i32::MIN);
    for v in vertices {
        min = min.min(IVec2::new(v.x, v.y));
        max = max.max(IVec2::new(v.x, v.y));
    }
    (min, max)
}

fn uv_or_zero(v: Vertex, pack_dx: f32, pack_dy: f32) -> Vec2 {
    use crate::game::config::windows::AUTO_UV;

    let u = v.u.unwrap_or(0);
    let v_ = v.v.unwrap_or(0);
    let u = if u == AUTO_UV { 0 } else { u };
    let v_ = if v_ == AUTO_UV { 0 } else { v_ };
    Vec2::new(u as f32 / pack_dx, v_ as f32 / pack_dy)
}
