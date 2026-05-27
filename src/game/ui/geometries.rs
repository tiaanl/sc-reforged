use bytemuck::NoUninit;
use glam::{IVec2, Vec2, Vec4};

use crate::{
    engine::storage::Handle,
    game::{
        render::textures::Texture,
        ui::{Rect, render::ui_mesh_renderer::UiMesh},
    },
};

use super::render::window_renderer::WindowRenderItems;

/// A single UI vertex in logical UI coordinates.
#[derive(Clone, Copy, NoUninit)]
#[repr(C)]
pub struct GeometryVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl GeometryVertex {
    fn new(pos: Vec2, uv: Vec2, color: Vec4) -> Self {
        Self {
            pos: pos.to_array(),
            uv: uv.to_array(),
            color: color.to_array(),
        }
    }
}

pub struct TiledGeometry {
    pub texture: Handle<Texture>,
    pub size: IVec2,
}

#[derive(Clone)]
pub struct BaseGeometry {
    pub mesh: UiMesh,
    pub texture: Handle<Texture>,
}

/// Each [Window] can have geometry rendered as backgrounds or UI elements.
#[derive(Default)]
pub struct Geometries {
    pub tiled: Vec<TiledGeometry>,
    pub base: Vec<BaseGeometry>,
}

impl Geometries {
    /// Clear out all the geometries so it can be populated again.
    pub fn clear(&mut self) {
        self.tiled.clear();
        self.base.clear();
    }

    pub fn render(&self, _position: IVec2, render_items: &mut WindowRenderItems) {
        self.render_tiled(render_items);
        self.render_base(render_items);
    }

    fn render_tiled(&self, render_items: &mut WindowRenderItems) {
        for geometry in self.tiled.iter() {
            render_items.render_textured_rect(
                Rect {
                    position: IVec2::ZERO,
                    size: geometry.size,
                },
                geometry.texture,
                Vec2::ZERO,
                Vec2::ONE,
                Vec4::ONE,
            );
        }
    }

    fn render_base(&self, _render_items: &mut WindowRenderItems) {}
}
