use bevy_ecs::prelude::*;

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

pub fn update_window_render_items(
    mut windows: Query<(&Window, &mut WindowRenderItems, &Children)>,
    tiled_geometry: Query<(&GeometryTiled, Option<&ZIndex>)>,
    sprites: Query<&SpriteRender>,
) {
    for (_window, mut window_render_items, children) in windows.iter_mut() {
        // Clear out old renders.
        window_render_items.clear();

        // Tiled Geometry
        {
            let mut tiled_geometries = tiled_geometry
                .iter_many(children)
                .map(|(tiled_geometry, z_index)| {
                    let z_index = z_index.map(|i| i.0).unwrap_or(0);

                    (
                        z_index,
                        tiled_geometry.tiled_geometry_handle,
                        tiled_geometry.alpha,
                    )
                })
                .collect::<Vec<_>>();
            tiled_geometries.sort_by_key(|(z_index, _, _)| *z_index);

            tiled_geometries
                .drain(..)
                .for_each(|(_, tiled_geometry_handle, alpha)| {
                    window_render_items.render_tiled_geometry(tiled_geometry_handle, alpha);
                });
        }

        // Widgets
        for sprite_render in sprites.iter_many(children) {
            window_render_items.render_sprite(
                sprite_render.position,
                sprite_render.sprite,
                sprite_render.frame,
                sprite_render.alpha,
            );
        }
    }
}
