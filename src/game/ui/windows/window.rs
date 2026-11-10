use std::{path::PathBuf, sync::Arc};

use glam::{IVec2, Vec2};

use crate::{
    engine::{assets::AssetError, renderer::RenderContext},
    game::{
        config::window_base as config,
        globals,
        ui::{
            EventResult, Rect,
            geometries::{Geometries, GeometryBase, GeometryTiled},
            render::{
                ui_mesh_renderer::{UiMesh, UiVertex},
                window_renderer::{WindowRenderItems, WindowRenderer},
            },
            widgets::widget::Widget,
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

pub trait WindowBehavior {
    /// Allow each [Window] to have its own update behavior.
    fn update(&mut self, window: &mut WindowCommon, delta: f32) {
        let _ = window;
        let _ = delta;
    }

    /// Allows each [Window] to have its own render behavior.
    fn render(
        &mut self,
        window: &mut WindowCommon,
        context: &mut WindowRenderContext<'_>,
        render_items: &mut WindowRenderItems,
    ) {
        window.render_geometries(render_items);
        window.render_widgets(context, render_items);
    }
}

struct DefaultWindowBehavior;

impl WindowBehavior for DefaultWindowBehavior {
    fn render(
        &mut self,
        window: &mut WindowCommon,
        context: &mut WindowRenderContext<'_>,
        render_items: &mut WindowRenderItems,
    ) {
        window.render_widgets(context, render_items)
    }
}

/// Data shared between all [Window]s.
pub struct WindowCommon {
    pub rect: Rect,

    pub visible: bool,
    pub enabled: bool,
    pub modal: bool,
    pub always_on_top: bool,

    pub window_base: Option<Arc<config::WindowBase>>,

    pub geometries: Geometries,

    pub widgets: Vec<Widget>,
    pub capture_widget: Option<usize>,
}

impl WindowCommon {
    pub fn render_geometries(&mut self, render_items: &mut WindowRenderItems) {
        self.geometries.render(self.rect.position, render_items);
    }

    pub fn render_widgets(
        &mut self,
        context: &mut WindowRenderContext,
        render_items: &mut WindowRenderItems,
    ) {
        for widget in self.widgets.iter_mut().rev() {
            widget.render(self.rect.position, context, render_items);
        }
    }
}

pub struct Window {
    pub common: WindowCommon,
    pub behavior: Box<dyn WindowBehavior>,
}

impl Window {
    /// Return true if the global coordinates are within the bounds of the
    /// window.
    pub fn hit_test(&self, position: IVec2) -> bool {
        self.common.rect.contains(position)
    }

    pub fn on_resize(&mut self, logical_size: IVec2) {
        if let Some(window_base) = &self.common.window_base {
            let layout_context = WindowLayoutContext::from_logical_size(logical_size);
            self.common.rect = window_base.resolve_layout_rect(&layout_context);

            if let Err(error) =
                populate_geometries(&mut self.common.geometries, window_base, &layout_context)
            {
                tracing::warn!(
                    "failed to refresh window base '{}' during resize: {error}",
                    window_base.name
                );
            }
        }
    }

    pub fn on_primary_mouse_down(
        &mut self,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        let Some(index) = self.top_level_input_widget(position) else {
            // If we are not over a top level widget that can receive input, we
            // ignore the event.
            return EventResult::Ignore;
        };

        // SAFETY: We can unwrap here, because `top_level_input_widget` only
        //         returns `Some` if the widget index was found.
        let widget = self.common.widgets.get_mut(index).unwrap();

        self.common.capture_widget = Some(index);

        if let Some(behavior) = widget.behavior.as_mut() {
            behavior.on_primary_mouse_down(&mut widget.common, position, context)
        } else {
            EventResult::Ignore
        }
    }

    pub fn on_primary_mouse_up(
        &mut self,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        if let Some(index) = self.common.capture_widget {
            if let Some(widget) = self.common.widgets.get_mut(index) {
                widget.on_primary_mouse_up(position, context);
            }
            self.common.capture_widget = None;
        }

        // for widget in self.common.widgets.iter_mut().rev() {
        //     let result = widget.on_primary_mouse_down(position, context);
        //     if !result.ignored() {
        //         return result;
        //     }
        // }

        // // Give the remaining widgets a chance to clear any pressed state even
        // // when the cursor was released outside their bounds.
        // for widget in self.common.widgets.iter_mut().rev() {
        //     // let result = widget.on_primary_mouse_up(position, context);

        //     // if matches!(result, EventResult::Handled) {
        //     //     return result;
        //     // }
        //     todo!()
        // }

        EventResult::Ignore
    }

    pub fn on_secondary_mouse_down(
        &mut self,
        _position: IVec2,
        _context: &mut WindowManagerContext,
    ) -> EventResult {
        EventResult::Ignore
    }

    pub fn on_secondary_mouse_up(
        &mut self,
        _position: IVec2,
        _context: &mut WindowManagerContext,
    ) -> EventResult {
        EventResult::Ignore
    }

    pub fn on_mouse_wheel(
        &mut self,
        _position: IVec2,
        _wheel_steps: i32,
        _context: &mut WindowManagerContext,
    ) -> EventResult {
        EventResult::Ignore
    }

    pub fn update(&mut self, _delta_time: f32) {}

    pub fn render(
        &mut self,
        context: &mut WindowRenderContext<'_>,
        render_items: &mut WindowRenderItems,
    ) {
        self.behavior
            .render(&mut self.common, context, render_items);
    }

    fn top_level_input_widget(&self, position: IVec2) -> Option<usize> {
        for (index, widget) in self.common.widgets.iter().enumerate().rev() {
            // Offset the widget position by the window position.

            if widget.common.visible
                && widget.common.accepts_input
                && widget
                    .common
                    .rect
                    .contains(self.common.rect.position + position)
            {
                return Some(index);
            }
        }

        None
    }
}

fn populate_geometries(
    geometries: &mut Geometries,
    window_base: &config::WindowBase,
    layout_context: &WindowLayoutContext,
) -> Result<(), AssetError> {
    geometries.clear();

    for geometry in window_base.geometries.iter() {
        match geometry {
            config::Geometry::Tiled(config::GeometryTiled {
                jpg_name,
                dimensions,
                ..
            }) => {
                let image = globals::images().load(
                    PathBuf::from("textures")
                        .join("interface")
                        .join(jpg_name)
                        .with_extension("jpg"),
                )?;

                let Some(texture) = globals::textures().create_from_image(image) else {
                    tracing::warn!("Frame JPG texture could not be loaded");
                    continue;
                };

                geometries.tiled.push(GeometryTiled {
                    texture,
                    size: *dimensions,
                });
            }

            config::Geometry::Base(geometry) => {
                let mesh = build_geometry_base_mesh(window_base, geometry, layout_context)?;
                geometries.base.push(GeometryBase { mesh });
            }
        }
    }

    Ok(())
}

fn build_geometry_base_mesh(
    window_base: &config::WindowBase,
    geometry: &config::GeometryBase,
    layout_context: &WindowLayoutContext,
) -> Result<UiMesh, AssetError> {
    let image = globals::images().load(
        PathBuf::from("textures")
            .join("interface")
            .join(&geometry.texture),
    )?;

    let texture = globals::textures()
        .create_from_image(image)
        .ok_or_else(|| {
            AssetError::Custom(
                PathBuf::from("textures")
                    .join("interface")
                    .join(&geometry.texture),
                String::from("Window base normal geometry texture could not be loaded"),
            )
        })?;

    let pack_dx = geometry.texture_pack_dx.max(1) as f32;
    let pack_dy = geometry.texture_pack_dy.max(1) as f32;

    let vertices = geometry
        .vertices
        .iter()
        .map(|vertex| {
            let x = window_base.resolve(&vertex.x, layout_context).unwrap_or(0) as f32;
            let y = window_base.resolve(&vertex.y, layout_context).unwrap_or(0) as f32;
            let u = vertex.u.unwrap_or(0) as f32 / pack_dx;
            let v = vertex.v.unwrap_or(0) as f32 / pack_dy;

            UiVertex {
                pos: [x, y],
                uv: [u, v],
                color: [vertex.r, vertex.g, vertex.b, vertex.a],
            }
        })
        .collect();

    let indices = geometry
        .polygons
        .iter()
        .flat_map(|polygon| [polygon.i0 as u32, polygon.i1 as u32, polygon.i2 as u32])
        .collect();

    Ok(UiMesh {
        vertices,
        indices,
        texture,
        clip_rect: None,
    })
}

fn offset_mesh(mesh: &mut UiMesh, offset: Vec2) {
    for vertex in mesh.vertices.iter_mut() {
        vertex.pos[0] += offset.x;
        vertex.pos[1] += offset.y;
    }
}

struct WindowBaseDetails<'a> {
    window_base: Arc<config::WindowBase>,
    context: &'a WindowLayoutContext,
}

pub struct WindowBuilder<'a> {
    rect: Rect,
    behavior: Option<Box<dyn WindowBehavior>>,
    window_base_details: Option<WindowBaseDetails<'a>>,
    modal: bool,
    always_on_top: bool,
}

impl<'a> WindowBuilder<'a> {
    pub fn new(rect: Rect) -> WindowBuilder<'a> {
        Self {
            rect,
            behavior: None,
            window_base_details: None,
            modal: false,
            always_on_top: false,
        }
    }

    pub fn with_behavior(mut self, behavior: Box<dyn WindowBehavior>) -> Self {
        self.behavior = Some(behavior);
        self
    }

    pub fn with_window_base(
        mut self,
        window_base: Arc<config::WindowBase>,
        context: &'a WindowLayoutContext,
    ) -> Self {
        self.window_base_details = Some(WindowBaseDetails {
            window_base,
            context,
        });
        self
    }

    pub fn modal(mut self, modal: bool) -> Self {
        self.modal = modal;
        self
    }

    pub fn always_on_top(mut self, always_on_top: bool) -> Self {
        self.always_on_top = always_on_top;
        self
    }

    pub fn build(self) -> Result<Window, AssetError> {
        let mut geometries = Geometries::default();
        let mut maybe_window_base = None;
        if let Some(WindowBaseDetails {
            window_base,
            context,
        }) = self.window_base_details
        {
            populate_geometries(&mut geometries, window_base.as_ref(), context)?;
            maybe_window_base = Some(window_base);
        }

        let behavior = self.behavior.unwrap_or(Box::new(DefaultWindowBehavior));

        let window = Window {
            common: WindowCommon {
                rect: self.rect,

                visible: true,
                enabled: true,
                modal: self.modal,
                always_on_top: self.always_on_top,

                window_base: maybe_window_base,
                geometries,

                widgets: Vec::default(),
                capture_widget: None,
            },
            behavior,
        };

        Ok(window)
    }
}
