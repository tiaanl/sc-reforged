#![allow(dead_code)]

use std::{path::PathBuf, sync::Arc, time::Instant};

use clap::Parser;

use glam::UVec2;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalPosition,
    event_loop::{ActiveEventLoop, EventLoop},
};

use crate::{
    engine::{
        input,
        renderer::{Gpu, RenderContext, RenderTarget, Surface, SurfaceDesc},
    },
    game::{game_state::GameState, globals},
};

mod engine;
mod game;

#[derive(clap::Parser)]
struct Opts {
    /// Path to the game data directory.
    /// (e.g. "C:\Program Files\Sinister Games\Shadow Comapany - Left for Dead\Data")
    path: PathBuf,
    /// The name of the starting campaign. Defaults to "training".
    campaign_name: Option<String>,
}

#[allow(clippy::large_enum_variant)]
enum App {
    Uninitialzed(Opts),
    Initialized {
        /// The main window the engine is rendering to. This is also the window
        /// that is receiving all the input events.
        window: Arc<winit::window::Window>,
        /// A description of the surface we can pass around.
        surface_desc: SurfaceDesc,
        /// The window surface where the scene will be displayed.
        surface: Surface,
        /// Our main [Gpu] holding the device and queue.
        gpu: Gpu,
        /// The index of the current frame being rendered.
        frame_index: u64,
        /// The instant that the last frame started to render.
        last_frame_time: Instant,
        /// egui integration.
        #[cfg(feature = "egui")]
        egui_integration: engine::egui_integration::EguiIntegration,

        /// The main state of the game.
        game_state: GameState,
    },
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        match self {
            App::Uninitialzed(opts) => {
                let mut attributes = winit::window::WindowAttributes::default()
                    .with_title("Shadow Company - Reforged")
                    .with_resizable(false)
                    .with_inner_size(winit::dpi::LogicalSize::new(640, 480));

                // No resizing?
                if true {
                    attributes = attributes.with_resizable(false).with_enabled_buttons(
                        winit::window::WindowButtons::MINIMIZE
                            | winit::window::WindowButtons::CLOSE,
                    )
                }

                if let Some(screen_size) =
                    event_loop.primary_monitor().map(|monitor| monitor.size())
                {
                    let position = winit::dpi::Position::Physical(PhysicalPosition::new(
                        screen_size.width as i32 / 4,
                        screen_size.height as i32 / 4,
                    ));
                    attributes = attributes.with_position(position);
                }

                let window = Arc::new(
                    event_loop
                        .create_window(attributes)
                        .expect("create main window"),
                );
                let _window_size = {
                    let winit::dpi::PhysicalSize { width, height } = window.inner_size();
                    UVec2::new(width, height)
                };

                let (surface, gpu) = engine::renderer::create(Arc::clone(&window));

                let surface_desc = SurfaceDesc {
                    size: surface.size(),
                    format: surface.format(),
                };

                #[cfg(feature = "egui")]
                let egui_integration = engine::egui_integration::EguiIntegration::new(
                    event_loop,
                    gpu.device.clone(),
                    gpu.queue.clone(),
                    surface_desc.format,
                );

                globals::init(&opts.path, gpu.clone());

                let game_state = match GameState::new(gpu.clone(), &surface_desc) {
                    Ok(game_state) => game_state,
                    Err(err) => {
                        tracing::error!("Could not initialize GameState - {err}");
                        event_loop.exit();
                        return;
                    }
                };

                tracing::info!("Application initialized!");

                *self = App::Initialized {
                    window,
                    surface,
                    surface_desc,
                    gpu,
                    #[cfg(feature = "egui")]
                    egui_integration,
                    frame_index: 0,
                    last_frame_time: Instant::now(),
                    game_state,
                };
            }

            App::Initialized { .. } => {
                tracing::warn!("Application already initialized!");
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        use winit::event::WindowEvent;
        match self {
            App::Uninitialzed(..) => {
                tracing::warn!("Can't process events for uninitialized application.");
            }
            App::Initialized {
                window,
                surface,
                surface_desc,
                gpu,
                frame_index,
                last_frame_time,
                #[cfg(feature = "egui")]
                egui_integration,
                game_state,
                ..
            } => {
                if window_id != window.id() {
                    return;
                }

                #[cfg(feature = "egui")]
                let repaint = {
                    let egui_winit::EventResponse { consumed, repaint } =
                        egui_integration.window_event(window.as_ref(), &event);
                    if consumed {
                        return;
                    }
                    repaint
                };

                match event {
                    WindowEvent::CloseRequested => {
                        event_loop.exit();
                    }

                    WindowEvent::Resized(winit::dpi::PhysicalSize { width, height }) => {
                        let size = UVec2::new(width, height);

                        surface.resize(&gpu.device, size);
                        surface_desc.size = surface.size();

                        game_state.resize(size);

                        window.request_redraw();
                    }

                    WindowEvent::RedrawRequested => {
                        let now = Instant::now();
                        let last_frame_duration = now - *last_frame_time;
                        *last_frame_time = now;

                        {
                            let delta_time = last_frame_duration.as_secs_f32();
                            game_state.update(delta_time);
                        }

                        {
                            let output = surface.get_texture(&gpu.device);
                            let surface_view = output
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());

                            let encoder = gpu.device.create_command_encoder(
                                &wgpu::CommandEncoderDescriptor {
                                    label: Some("main command encoder"),
                                },
                            );

                            let mut render_context = RenderContext {
                                encoder,
                                frame_index: *frame_index,
                            };
                            let render_target = RenderTarget {
                                view: surface_view,
                                size: surface.size(),
                            };

                            game_state.render(&mut render_context, &render_target);

                            // Render egui if it requires a repaint.
                            #[cfg(feature = "egui")]
                            if repaint {
                                egui_integration.render(
                                    window,
                                    &mut render_context.encoder,
                                    &render_target.view,
                                    |ctx| {
                                        ctx.set_pixels_per_point(1.2);
                                        // Debug stuff from the scene.
                                        game_state.debug_panel(ctx, render_context.frame_index);
                                    },
                                );
                            }

                            gpu.queue
                                .submit(std::iter::once(render_context.encoder.finish()));

                            output.present();

                            // Frame is done rendering.

                            *frame_index += 1;

                            window.request_redraw();
                        }
                    }

                    _ => {}
                }

                if let Some(input_event) = input::translate_window_event(&event) {
                    game_state.input(&input_event);
                }
            }
        }
    }
}

fn main() {
    tracing_subscriber::fmt().init();

    let opts = match Opts::try_parse() {
        Ok(opts) => opts,
        Err(err) => {
            err.print().unwrap();
            return;
        }
    };

    let event_loop = EventLoop::new().unwrap();

    let mut app = App::Uninitialzed(opts);
    event_loop
        .run_app(&mut app)
        .expect("run application event loop");
}
