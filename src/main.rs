#![allow(dead_code)]

use std::{path::PathBuf, sync::Arc, time::Instant};

use clap::Parser;

use glam::UVec2;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event_loop::{ActiveEventLoop, EventLoop},
};

use crate::{
    engine::{
        input,
        renderer::{RenderContext, RenderWindow},
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
        /// The window the engine renders to, which is also the window receiving
        /// all the input events.
        render_window: RenderWindow,
        /// The index of the current frame being rendered.
        frame_index: u64,
        /// The instant that the last frame started to render.
        last_frame_time: Instant,

        /// The main state of the game.
        game_state: GameState,
    },
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        match self {
            App::Uninitialzed(opts) => {
                let attributes = winit::window::WindowAttributes::default()
                    .with_title("Shadow Company - Reforged")
                    .with_resizable(true)
                    .with_inner_size(winit::dpi::LogicalSize::new(800, 600));

                let window = Arc::new(
                    event_loop
                        .create_window(attributes)
                        .expect("create main window"),
                );
                let (render_window, gpu) = RenderWindow::new(window);

                globals::init(&opts.path, gpu);

                let game_state = match GameState::new(&render_window.desc()) {
                    Ok(game_state) => game_state,
                    Err(err) => {
                        tracing::error!("Could not initialize GameState - {err}");
                        event_loop.exit();
                        return;
                    }
                };

                tracing::info!("Application initialized!");

                *self = App::Initialized {
                    render_window,
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
                render_window,
                frame_index,
                last_frame_time,
                game_state,
                ..
            } => {
                if !render_window.has_id(window_id) {
                    return;
                }

                match event {
                    WindowEvent::CloseRequested => {
                        event_loop.exit();
                    }

                    WindowEvent::Resized(PhysicalSize { width, height }) => {
                        let size = UVec2::new(width, height);

                        render_window.resize(&globals::gpu().device, size);
                        game_state.resize(size, render_window.desc().scale_factor);

                        render_window.request_redraw();
                    }

                    WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                        game_state.resize(render_window.desc().size, scale_factor as f32);
                        render_window.request_redraw();
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
                            if let Some(frame) = render_window.next_frame(&globals::gpu().device) {
                                let encoder = globals::gpu().device.create_command_encoder(
                                    &wgpu::CommandEncoderDescriptor {
                                        label: Some("main command encoder"),
                                    },
                                );

                                let mut render_context = RenderContext {
                                    encoder,
                                    frame_index: *frame_index,
                                };

                                game_state.render(&mut render_context, &frame.target);

                                globals::gpu()
                                    .queue
                                    .submit(std::iter::once(render_context.encoder.finish()));

                                frame.present(&globals::gpu().queue);

                                *frame_index += 1;
                            }

                            render_window.request_redraw();
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
