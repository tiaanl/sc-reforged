use std::sync::Arc;

use glam::{UVec2, Vec2};

use crate::{
    engine::{
        renderer::{Frame, RenderContext, SurfaceDesc},
        storage::{Handle, Storage},
    },
    game::{
        assets::image::Image,
        render::textures::{Texture, Textures},
        scenes::main_menu::quad_renderer::{Quad, QuadRenderer},
    },
};

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
    pub fn render_sprite(&mut self) {
        self.0.push(RenderItem::Sprite);
    }
}

/// Renders all the components required for windows.
pub struct WindowRenderer {
    textures: Arc<Textures>,
    quad_renderer: QuadRenderer,
    tiled_geometries: Storage<TiledGeometry>,
}

impl WindowRenderer {
    /// Creates the window renderer.
    pub fn new(
        render_context: RenderContext,
        surface_desc: &SurfaceDesc,
        textures: Arc<Textures>,
    ) -> Self {
        Self {
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
                        pos: UVec2::ZERO,
                        size: geometry.render_size,
                        texture: geometry.texture,
                        alpha: *alpha,
                        uv_min: Vec2::ZERO,
                        uv_max: Vec2::ONE,
                    });
                }
                RenderItem::Sprite => {}
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
    Sprite,
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
