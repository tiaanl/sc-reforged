use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use ahash::HashMap;
use glam::IVec2;
use winit::event::MouseButton;

use crate::{
    engine::{
        assets::AssetError,
        input::InputEvent,
        renderer::{RenderContext, RenderTarget, SurfaceDesc},
    },
    game::{
        config::{load_config, windows::WindowBase},
        ui::{
            EventResult,
            render::window_renderer::{WindowRenderItems, WindowRenderer},
            windows::window_manager_context::WindowManagerContext,
        },
    },
};

use super::window::{Window, WindowRenderContext};

pub struct WindowManager {
    window_bases: Mutex<HashMap<String, Arc<WindowBase>>>,

    window_renderer: WindowRenderer,
    window_render_items_cache: WindowRenderItems,

    pub window_manager_context: WindowManagerContext,

    /// The stack of windows. In bottom-to-top z-index order.
    windows: Vec<Box<dyn Window>>,
    /// The index of the current modal window in `windows`, if any.
    modal_window: Option<usize>,

    /// The current position of the mouse in the native window. None if the
    /// mouse left the window at some point.
    mouse_position: Option<IVec2>,

    /// Track the down state of the used mouse buttons.
    primary_button_down: bool,
    secondary_button_down: bool,
}

impl WindowManager {
    pub fn new(surface_desc: &SurfaceDesc) -> Result<Self, AssetError> {
        let window_renderer = WindowRenderer::new(surface_desc);

        Ok(Self {
            window_bases: Mutex::new(HashMap::default()),

            window_renderer,
            window_render_items_cache: WindowRenderItems::default(),

            window_manager_context: WindowManagerContext::default(),

            windows: Vec::default(),
            modal_window: None,

            mouse_position: None,
            primary_button_down: false,
            secondary_button_down: false,
        })
    }

    pub fn window_renderer(&self) -> &WindowRenderer {
        &self.window_renderer
    }

    pub fn get_window_base(&self, name: &str) -> Result<Arc<WindowBase>, AssetError> {
        if let Some(def) = self.window_bases.lock().unwrap().get(name).cloned() {
            return Ok(def);
        }

        let path = PathBuf::from("config")
            .join("window_bases")
            .join(name)
            .with_extension("txt");

        let loaded: Arc<WindowBase> = Arc::new(load_config(path)?);

        let mut defs = self.window_bases.lock().unwrap();
        let def = defs
            .entry(name.to_string())
            .or_insert_with(|| Arc::clone(&loaded))
            .clone();

        Ok(def)
    }

    /// Clear all windows.
    pub fn clear(&mut self) {
        self.windows.clear();
    }

    /// Push a new window to the top of the stack.
    pub fn push(&mut self, window: Box<dyn Window>) {
        let is_modal = window.is_modal();
        let is_always_on_top = window.is_always_on_top();

        let insert_index = if is_modal || is_always_on_top {
            self.windows.len()
        } else {
            self.windows
                .iter()
                .position(|window| window.is_always_on_top())
                .unwrap_or(self.windows.len())
        };

        if let Some(modal_index) = self.modal_window
            && insert_index > modal_index
        {
            panic!("cannot insert a window above a modal window");
        }

        if is_modal && self.modal_window.is_some() {
            panic!("cannot push a modal window while another modal window exists");
        }

        self.windows.insert(insert_index, window);

        if let Some(modal_index) = &mut self.modal_window
            && insert_index <= *modal_index
        {
            *modal_index += 1;
        }

        if is_modal {
            self.modal_window = Some(insert_index);
        }
    }

    pub fn resize(&mut self, size: glam::UVec2) {
        self.window_renderer.resize(size);
    }

    pub fn input(&mut self, event: &InputEvent) {
        match *event {
            InputEvent::MouseMove(position) => {
                self.mouse_position = Some(position.as_ivec2());
            }
            InputEvent::MouseLeave => {
                self.mouse_position = None;
            }
            InputEvent::MouseDown(button) => {
                //
                self.dispatch_mouse_down(button)
            }
            InputEvent::MouseUp(button) => self.dispatch_mouse_up(button),
            InputEvent::KeyDown(_key) => {}
            InputEvent::KeyUp(_key) => {}
            InputEvent::MouseWheel(delta) => self.dispatch_mouse_wheel(delta as i32),
        }

        // Fan out the raw event to any window that wants direct access to the
        // input stream (e.g. WorldWindow forwarding to its simulation).
        for window in self.windows.iter_mut() {
            window.on_input(event);
        }
    }

    fn dispatch_mouse_down(&mut self, button: MouseButton) {
        let Some(mouse) = self.mouse_position else {
            return;
        };

        match button {
            MouseButton::Left => self.primary_button_down = true,
            MouseButton::Right => self.secondary_button_down = true,
            _ => {}
        }

        if let Some(modal_index) = self.modal_window
            && let Some(window) = self.windows.get_mut(modal_index)
        {
            Self::try_mouse_down_on_window(
                window.as_mut(),
                mouse,
                button,
                &mut self.window_manager_context,
            );
            return;
        }

        let windows = &mut self.windows;
        for window in windows.iter_mut().rev() {
            let result = Self::try_mouse_down_on_window(
                window.as_mut(),
                mouse,
                button,
                &mut self.window_manager_context,
            );

            if matches!(result, EventResult::Handled) {
                return;
            }

            // TODO: Handle dragging and capture results.

            /*
            switch (result) {
            case Begin_Window_Drag:
                m_mouse_down_window = window;
                m_mouse_down_mode = Drag_Window;
                m_drag_start_local_x = local_x;
                m_drag_start_local_y = local_y;
                Bring_Window_To_Front(true);
                return true;

            case Capture_Mouse:
            case Capture_Mouse_Alternate:
                m_mouse_down_window = window;
                m_mouse_down_mode = Captured_Mouse;
                Bring_Window_To_Front(true);
                return true;

            case Capture_Mouse_No_Focus:
                m_mouse_down_window = window;
                m_mouse_down_mode = Special_Capture;
                Bring_Window_To_Front(false);
                return true;

            default:
                // Even if no capture/drag state is entered,
                // the click was still consumed by this window.
                return true;
            }
            */
        }
    }

    fn try_mouse_down_on_window(
        window: &mut dyn Window,
        mouse: IVec2,
        button: MouseButton,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        if !window.is_visible() || !window.wants_input() || !window.hit_test(mouse) {
            return EventResult::Ignore;
        }

        let local = mouse - window.rect().position;

        println!("local: {local}");

        match button {
            MouseButton::Left => window.on_primary_mouse_down(local, context),
            MouseButton::Right => window.on_secondary_mouse_down(local, context),
            _ => EventResult::Ignore,
        }
    }

    fn dispatch_mouse_up(&mut self, button: MouseButton) {
        match button {
            MouseButton::Left => self.primary_button_down = false,
            MouseButton::Right => self.secondary_button_down = false,
            _ => {}
        }

        let Some(mouse) = self.mouse_position else {
            return;
        };

        // TODO: Match the original manager more closely once we track mouse
        // capture / drag state. In Ghidra the mouse-up path is routed through
        // the captured or mouse-down window, not by re-hit-testing the current
        // topmost window under the cursor.
        let Some(window_index) = self.topmost_input_window_index(mouse) else {
            return;
        };

        let window = self.windows[window_index].as_mut();
        let local = mouse - window.rect().position;

        let context = &mut self.window_manager_context;

        match button {
            MouseButton::Left => {
                let _ = window.on_primary_mouse_up(local, context);
            }
            MouseButton::Right => {
                let _ = window.on_secondary_mouse_up(local, context);
            }
            _ => {}
        }

        // TODO: Clear transient drag items and finalize any captured-mouse
        // state like the original manager does after mouse-up.
    }

    fn dispatch_mouse_wheel(&mut self, delta: i32) {
        // In Ghidra the wheel goes to the captured window if one exists,
        // otherwise to the hovered window. We do not track either yet, so use
        // the current topmost eligible window under the cursor as a first pass.
        let Some(mouse) = self.mouse_position else {
            return;
        };

        let Some(window_index) = self.topmost_input_window_index(mouse) else {
            return;
        };

        let window = self.windows[window_index].as_mut();
        let local = mouse - window.rect().position;

        let context = &mut self.window_manager_context;
        let _ = window.on_mouse_wheel(local, delta, context);

        // TODO: Route wheel input to the captured or hovered window once the
        // manager tracks those states explicitly.
    }

    fn topmost_input_window_index(&self, mouse: IVec2) -> Option<usize> {
        if let Some(index) = self.modal_window {
            let window = &self.windows[index];
            return (window.is_visible() && window.wants_input() && window.hit_test(mouse))
                .then_some(index);
        }

        self.windows
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, window)| {
                (window.is_visible() && window.wants_input() && window.hit_test(mouse))
                    .then_some(index)
            })
    }

    pub fn update(&mut self, delta_time: f32) {
        for window in self.windows.iter_mut() {
            window.update(delta_time);
        }
    }

    pub fn render(&mut self, render_context: &mut RenderContext, render_target: &RenderTarget) {
        self.window_render_items_cache.clear();

        let mut ctx = WindowRenderContext {
            render_context,
            window_renderer: &self.window_renderer,
        };
        for window in self.windows.iter_mut() {
            window.render(&mut ctx, &mut self.window_render_items_cache);
        }

        self.window_renderer.submit_render_items(
            render_context,
            render_target,
            &self.window_render_items_cache,
        );
    }
}
