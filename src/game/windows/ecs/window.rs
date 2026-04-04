use bevy_ecs::prelude::*;
use glam::Vec4;

use crate::game::windows::{
    ecs::{ZIndex, geometry::GeometryTiled, rect::Rect, render::SpriteRender},
    window_renderer::WindowRenderItems,
};

#[derive(Component, Default)]
#[require(Rect)]
#[require(WindowRenderItems)]
pub struct Window;

#[derive(Default, Resource)]
pub struct WindowManager {
    pub windows: Vec<Entity>,
}

impl WindowManager {
    /// Push a new window to the top of the stack.
    pub fn push(&mut self, window: Entity) {
        tracing::info!("Pushing window: {window}");
        self.windows.push(window);
    }

    /// Remove a window from the stack. Note: Doesn't have to be the top-most
    /// window.
    pub fn remove(&mut self, window: Entity) -> bool {
        tracing::info!("Removing window: {window}");
        self.windows
            .iter()
            .position(|&e| e == window)
            .map(|index| {
                self.windows.remove(index);
            })
            .is_some()
    }
}

/// Rebuilds each window's queued render items from its descendant geometry and
/// sprite components. Windows without tiled geometry fall back to a simple
/// border-only chrome item.
pub fn update_window_render_items(
    mut windows: Query<(Entity, &Rect, &mut WindowRenderItems), With<Window>>,
    hierarchy: Query<&Children>,
    tiled_geometry: Query<(&GeometryTiled, Option<&ZIndex>)>,
    sprites: Query<&SpriteRender>,
    mut descendants: Local<Vec<Entity>>,
) {
    for (window, rect, mut window_render_items) in windows.iter_mut() {
        // Clear out old renders.
        window_render_items.clear();
        descendants.clear();
        descendants.extend(hierarchy.iter_descendants::<Children>(window));

        // Tiled Geometry
        let mut tiled_geometries = descendants
            .iter()
            .filter_map(|&entity| {
                tiled_geometry
                    .get(entity)
                    .ok()
                    .map(|(tiled_geometry, z_index)| {
                        let z_index = z_index.map(|i| i.0).unwrap_or(0);

                        (
                            z_index,
                            tiled_geometry.tiled_geometry_handle,
                            tiled_geometry.alpha,
                        )
                    })
            })
            .collect::<Vec<_>>();

        if tiled_geometries.is_empty() {
            // Render a default window background.
            window_render_items.render_border(rect.position, rect.size, 2, Vec4::ONE);
        } else {
            tiled_geometries.sort_by_key(|(z_index, _, _)| *z_index);

            tiled_geometries
                .drain(..)
                .for_each(|(_, tiled_geometry_handle, alpha)| {
                    window_render_items.render_tiled_geometry(tiled_geometry_handle, alpha);
                });
        }

        // Widgets
        for &entity in &descendants {
            if let Ok(sprite_render) = sprites.get(entity) {
                window_render_items.render_sprite(
                    sprite_render.position,
                    sprite_render.sprite,
                    sprite_render.frame,
                    sprite_render.alpha,
                );
            }
        }
    }
}
