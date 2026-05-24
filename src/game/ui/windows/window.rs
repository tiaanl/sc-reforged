use std::{path::PathBuf, sync::Arc};

use glam::{IVec2, Vec2, Vec4};

use crate::{
    engine::{assets::AssetError, renderer::RenderContext, storage::Handle},
    game::{
        assets::sprites::Sprite3d,
        config::window_base::WindowBase,
        globals,
        render::textures::Texture,
        ui::{
            EventResult, Rect,
            render::window_renderer::{WindowRenderItems, WindowRenderer},
            widgets::widget::Widgets,
            windows::{
                window_manager::WindowLayoutContext, window_manager_context::WindowManagerContext,
            },
        },
    },
};

/// Resources passed to each window during the render phase. Carries enough to
/// drive direct GPU work (e.g. world rendering into a gbuffer) in addition to
/// emitting window render items.
pub struct WindowRenderContext<'a> {
    pub render_context: &'a mut RenderContext,
    pub window_renderer: &'a WindowRenderer,
}

pub struct WindowCommon {
    pub rect: Rect,

    //pub geometries: Geometries,
    pub render_geometry: RenderGeometry,
    pub widgets: Widgets,

    pub is_visible: bool,
    pub is_enabled: bool,
    pub is_modal: bool,
    pub is_always_on_top: bool,
}

impl WindowCommon {
    pub fn new(rect: Rect) -> Self {
        Self {
            rect,
            // geometries: Geometries::default(),
            render_geometry: RenderGeometry::default(),
            widgets: Widgets::default(),

            is_visible: true,
            is_enabled: true,
            is_modal: false,
            is_always_on_top: false,
        }
    }
}

pub trait WindowImpl {
    /// Handle a primary mouse down event.
    fn on_primary_mouse_down(
        &mut self,
        common: &mut WindowCommon,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let _ = common;
        let _ = position;
        let _ = context;
        EventResult::Ignore
    }

    /// Handle a secondary mouse down event.
    fn on_secondary_mouse_down(
        &mut self,
        common: &mut WindowCommon,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let _ = common;
        let _ = position;
        let _ = context;
        EventResult::Ignore
    }

    /// Handle a primary mouse up event.
    fn on_primary_mouse_up(
        &mut self,
        common: &mut WindowCommon,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let _ = common;
        let _ = position;
        let _ = context;
        EventResult::Ignore
    }

    /// Handle a secondary mouse up event.
    fn on_secondary_mouse_up(
        &mut self,
        common: &mut WindowCommon,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let _ = common;
        let _ = position;
        let _ = context;
        EventResult::Ignore
    }

    /// Handle a mouse wheel event in window-local coordinates.
    fn on_mouse_wheel(
        &mut self,
        common: &mut WindowCommon,
        position: IVec2,
        wheel_steps: i32,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let _ = common;
        let _ = position;
        let _ = wheel_steps;
        let _ = context;
        EventResult::Ignore
    }

    /// Called to update the window state given the time in seconds since the last frame was drawn
    /// in `delta_time`.
    fn update(&mut self, delta_time: f32) {
        let _ = delta_time;
    }

    /// Called when the logical UI size changes so windows can re-resolve any
    /// expression-based layout (button positions, geometry, etc.) keyed off
    /// `%screen_dx` / `%screen_dy`.
    fn on_resize(&mut self, common: &mut WindowCommon, logical_size: IVec2) {
        let _ = common;
        let _ = logical_size;
    }

    /// Called for each window so they can drive any GPU work and append items
    /// to `render_items` to be composited later.
    ///
    /// The default implementation renders `common.geometries` (background art
    /// from the window base) followed by `common.widgets`. Overrides are
    /// responsible for calling these themselves if they want them drawn.
    fn render(
        &mut self,
        common: &mut WindowCommon,
        context: &mut WindowRenderContext<'_>,
        render_items: &mut WindowRenderItems,
    ) {
        // common.geometries.render(common.rect.position, render_items);
        common
            .widgets
            .render(common.rect.position, 0, context, render_items);
    }
}

pub struct Window {
    pub window_base: Option<Arc<WindowBase>>,
    pub common: WindowCommon,
    pub window_impl: Box<dyn WindowImpl>,
}

impl Window {
    pub fn new(common: WindowCommon, window_impl: Box<dyn WindowImpl>) -> Self {
        Self {
            window_base: None,
            common,
            window_impl,
        }
    }

    pub fn from_window_base(
        window_base: Arc<WindowBase>,
        rect: Rect,
        window_impl: Box<dyn WindowImpl + 'static>,
    ) -> Result<Self, AssetError> {
        let mut common = WindowCommon::new(rect);

        populate_geometries(&mut common.render_geometry, &window_base)?;

        Ok(Self {
            window_base: Some(window_base),
            common,
            window_impl,
        })
    }

    /// Return true if the window is visible.
    pub fn is_visible(&self) -> bool {
        self.common.is_visible
    }

    /// Return true if the window can receive input events.
    pub fn is_enabled(&self) -> bool {
        self.common.is_enabled
    }

    /// Return true if the window is modal and should exclusively receive input.
    pub fn is_modal(&self) -> bool {
        self.common.is_modal
    }

    /// Return true if the window should stay above normal windows.
    pub fn is_always_on_top(&self) -> bool {
        self.common.is_always_on_top
    }

    /// Return true if the global coordinates are within the bounds of the
    /// window.
    pub fn hit_test(&self, position: IVec2) -> bool {
        self.common.rect.contains(position)
    }

    /// Return the [Rect] of the window.
    pub fn rect(&self) -> Rect {
        self.common.rect
    }

    pub fn on_resize(&mut self, logical_size: IVec2) {
        if let Some(window_base) = &self.window_base {
            let layout_context = WindowLayoutContext::from_logical_size(logical_size);
            self.common.rect = window_base.resolve_layout_rect(&layout_context);

            if let Err(error) = populate_geometries(&mut self.common.render_geometry, window_base) {
                tracing::warn!(
                    "failed to refresh window base '{}' during resize: {error}",
                    window_base.name
                );
            }
        }

        self.window_impl.on_resize(&mut self.common, logical_size);
    }

    pub fn on_primary_mouse_down(
        &mut self,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let result = self.common.widgets.on_primary_mouse_down(position, context);
        if matches!(result, EventResult::Handled) {
            return result;
        }
        self.window_impl
            .on_primary_mouse_down(&mut self.common, position, context)
    }

    pub fn on_primary_mouse_up(
        &mut self,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let result = self.common.widgets.on_primary_mouse_up(position, context);
        if matches!(result, EventResult::Handled) {
            return result;
        }
        self.window_impl
            .on_primary_mouse_up(&mut self.common, position, context)
    }

    pub fn on_secondary_mouse_down(
        &mut self,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let result = self
            .common
            .widgets
            .on_secondary_mouse_down(position, context);
        if matches!(result, EventResult::Handled) {
            return result;
        }
        self.window_impl
            .on_secondary_mouse_down(&mut self.common, position, context)
    }

    pub fn on_secondary_mouse_up(
        &mut self,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let result = self.common.widgets.on_secondary_mouse_up(position, context);
        if matches!(result, EventResult::Handled) {
            return result;
        }
        self.window_impl
            .on_secondary_mouse_up(&mut self.common, position, context)
    }

    pub fn on_mouse_wheel(
        &mut self,
        position: IVec2,
        wheel_steps: i32,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let result = self
            .common
            .widgets
            .on_mouse_wheel(position, wheel_steps, context);
        if matches!(result, EventResult::Handled) {
            return result;
        }
        self.window_impl
            .on_mouse_wheel(&mut self.common, position, wheel_steps, context)
    }

    pub fn render(
        &mut self,
        context: &mut WindowRenderContext<'_>,
        render_items: &mut WindowRenderItems,
    ) {
        self.window_impl
            .render(&mut self.common, context, render_items);
    }

    pub fn update(&mut self, delta_time: f32) {
        self.window_impl.update(delta_time);
    }
}

#[derive(Debug)]
pub struct TiledRenderGeometry {
    rect: Rect,
    texture: Handle<Texture>,
}

#[derive(Debug)]
pub struct NormalRenderGeometry {
    pos: IVec2,
    sprite: Handle<Sprite3d>,
    frame: i32,
}

#[derive(Debug, Default)]
pub struct RenderGeometry {
    tiled: Vec<TiledRenderGeometry>,
    normal: Vec<NormalRenderGeometry>,
}

impl RenderGeometry {
    /// Queues the window-base geometries at the specified UI-space origin.
    pub fn render(&self, origin: IVec2, render_items: &mut WindowRenderItems) {
        for geometry in self.tiled.iter() {
            render_items.render_textured_rect(
                geometry.rect.offset(origin),
                geometry.texture,
                Vec2::ZERO,
                Vec2::ONE,
                Vec4::ONE,
            );
        }

        for geometry in self.normal.iter() {
            render_items.render_sprite(geometry.pos, geometry.sprite, geometry.frame as usize, 1.0);
        }
    }
}

fn populate_geometries(
    geometries: &mut RenderGeometry,
    window_base: &WindowBase,
) -> Result<(), AssetError> {
    geometries.tiled.clear();

    for geometry in window_base.geometries.iter() {
        use crate::game::config::window_base::Geometry as G;

        match geometry {
            G::Normal(_geometry) => {
                // geometries.normal.push({
                //     NormalRenderGeometry {
                //         pos: geometry.,
                //         sprite: todo!(),
                //         frame: todo!(),
                //     }
                // });
            }

            G::Tiled(tiled) => geometries.tiled.push({
                let image = globals::images().load(
                    PathBuf::from("textures")
                        .join("interface")
                        .join(&tiled.jpg_name)
                        .with_extension("jpg"),
                )?;

                let Some(texture) = globals::textures().create_from_image(image) else {
                    tracing::warn!("Frame JPG texture could not be loaded");
                    continue;
                };

                TiledRenderGeometry {
                    rect: Rect::from_size(tiled.dimensions),
                    texture,
                }
            }),
        }
    }

    Ok(())
}
