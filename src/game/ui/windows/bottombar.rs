use std::{path::PathBuf, sync::Arc};

use glam::{IVec2, Vec2, Vec4};

use crate::{
    engine::{assets::AssetError, storage::Handle},
    game::{
        config::windows::{
            Geometry, GeometryNormal, Vertex, WindowBase, WindowBaseLayout, WindowCtx,
        },
        globals,
        render::textures::Texture,
        ui::{
            Rect,
            render::window_renderer::WindowRenderItems,
            windows::window::{Window, WindowCommon, WindowImpl, WindowRenderContext},
        },
    },
};

/// The in-game bottom bar containing the radar, clock, and command buttons.
///
/// Mirrors the original engine's `bottombar_640x480` window base. The 800x600
/// variant is not implemented yet; switching variants requires runtime
/// `%screen_dx`/`%screen_dy` resolution.
pub struct BottomBarWindow {
    window_base: Arc<WindowBase>,
    layout: WindowBaseLayout,
    geometry_textures: Vec<Option<Handle<Texture>>>,
}

impl BottomBarWindow {
    pub fn new(surface_size: IVec2) -> Result<Window, AssetError> {
        // Matches the original engine's selection rule: the 640x480 base is
        // only used on an exact 640x480 surface, otherwise the 800x600 base is
        // the default. Both share WINDOW_BASE_DY 194 and use %screen_dx to fill
        // the screen width, so the same resolver works for either.
        let window_base = globals::window_manager().get_window_base("bottombar_800x600")?;
        let layout = window_base.layout(&WindowCtx::from_logical_size(surface_size));

        let geometry_textures = layout
            .geometries
            .iter()
            .map(|geometry| match geometry {
                Geometry::Normal(normal) => load_texture(&normal.texture),
                Geometry::Tiled(_) => None,
            })
            .collect();

        let common = WindowCommon::new(compute_rect(&layout, surface_size));

        Ok(Window::new(
            common,
            Box::new(Self {
                window_base,
                layout,
                geometry_textures,
            }),
        ))
    }
}

/// Places the bar flush against the bottom of the screen, spanning the full
/// window-base width — matches the original engine layout.
fn compute_rect(layout: &WindowBaseLayout, logical: IVec2) -> Rect {
    Rect::new(
        IVec2::new(0, logical.y - layout.dy),
        IVec2::new(layout.dx, layout.dy),
    )
}

impl WindowImpl for BottomBarWindow {
    fn on_resize(&mut self, common: &mut WindowCommon, logical_size: IVec2) {
        self.layout = self
            .window_base
            .layout(&WindowCtx::from_logical_size(logical_size));
        common.rect = compute_rect(&self.layout, logical_size);
    }

    fn render(
        &mut self,
        common: &mut WindowCommon,
        _context: &mut WindowRenderContext<'_>,
        render_items: &mut WindowRenderItems,
    ) {
        for (geometry, texture) in self
            .layout
            .geometries
            .iter()
            .zip(self.geometry_textures.iter())
        {
            let Geometry::Normal(normal) = geometry else {
                continue;
            };
            let Some(texture) = *texture else {
                continue;
            };

            let Some((rect, uv_min, uv_max, color)) = compute_quad(normal) else {
                continue;
            };

            // Translate the geometry from window-base local coordinates into
            // global UI coordinates by offsetting with the window position.
            let global_rect = rect.offset(common.rect.position);

            render_items.render_textured_rect(global_rect, texture, uv_min, uv_max, color);
        }
    }
}

/// Resolves a window-base geometry texture name (e.g. `simu1_ck.bmp`) to a
/// loaded texture handle. Today we only look in `textures/interface/`, which
/// covers every texture referenced by the bottombar bases.
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
/// color and UV interpolation; leaving that for when we wire up real widgets.
fn compute_quad(geometry: &GeometryNormal) -> Option<(Rect, Vec2, Vec2, Vec4)> {
    if geometry.vertices.len() < 4 {
        return None;
    }

    let (min, max) = bounding_box(&geometry.vertices);
    let rect = Rect::new(min, max - min);

    // Use vertex 0 (top-left in the source files) for color.
    let v0 = geometry.vertices[0];
    let color = Vec4::new(v0.r, v0.g, v0.b, v0.a);

    let pack_dx = geometry.texture_pack_dx.max(1) as f32;
    let pack_dy = geometry.texture_pack_dy.max(1) as f32;

    // Pull UVs from the bbox corners. AUTO is treated as 0 for now.
    let uv0 = uv_or_zero(geometry.vertices[0], pack_dx, pack_dy);
    let uv2 = uv_or_zero(geometry.vertices[2], pack_dx, pack_dy);
    let uv_min = uv0.min(uv2);
    let uv_max = uv0.max(uv2);

    Some((rect, uv_min, uv_max, color))
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
