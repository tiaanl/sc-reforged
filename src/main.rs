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
        renderer::{Frame, RenderContext, Surface, SurfaceDesc},
        scene::Scene,
        threads::main::{MainThreadEvent, MainThreadReceiver},
    },
    game::{
        assets::config::campaign_def::CampaignDefs, config::load_config, file_system::FileSystem,
        scenes::main_menu::MainMenuScene,
    },
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
    Uninitialzed(Opts, MainThreadReceiver),
    Initialized {
        /// The main window the engine is rendering to. This is also the window
        /// that is receiving all the input events.
        window: Arc<winit::window::Window>,
        /// A description of the surface we can pass around.
        surface_desc: SurfaceDesc,
        /// The window surface where the scene will be displayed.
        surface: Surface,
        /// Our main [RenderContext] holding the device and queue.
        context: RenderContext,
        /// The index of the current frame being rendered.
        frame_index: u64,
        /// The instant that the last frame started to render.
        last_frame_time: Instant,
        /// egui integration.
        #[cfg(feature = "egui")]
        egui_integration: engine::egui_integration::EguiIntegration,
        /// The scene we are currently rendering to the screen.
        scene: Box<dyn Scene>,
        /// A new scene requested to be switched to.
        requested_scene: Option<Box<dyn Scene>>,

        /// A receiver for events on the main thread.
        _main_thread_receiver: MainThreadReceiver,
    },
}

impl ApplicationHandler<MainThreadEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        match self {
            App::Uninitialzed(opts, main_thread_receiver) => {
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

                let (surface, context) = engine::renderer::create(Arc::clone(&window));

                let surface_desc = SurfaceDesc {
                    size: surface.size(),
                    format: surface.format(),
                };

                #[cfg(feature = "egui")]
                let egui_integration = engine::egui_integration::EguiIntegration::new(
                    event_loop,
                    context.device.clone(),
                    context.queue.clone(),
                    surface_desc.format,
                );

                let file_system = Arc::new(FileSystem::new(&opts.path));

                /*
                let scene: Box<dyn Scene> = {
                    let campaign_name = opts
                        .campaign_name
                        .clone()
                        .unwrap_or(String::from("training"));

                    let campaign_defs = load_config::<CampaignDefs>(
                        &file_system,
                        PathBuf::from("config").join("campaign_defs.txt"),
                    )
                    .unwrap();

                    let campaign_def = campaign_defs
                        .campaigns
                        .iter()
                        .find(|c| c.base_name == campaign_name)
                        .cloned()
                        .unwrap();

                    Box::new(
                        WorldScene::new(
                            &file_system,
                            &renderer,
                            surface.size(),
                            surface.format(),
                            campaign_def,
                        )
                        .unwrap(),
                    )
                };
                */

                let _campaign_defs: CampaignDefs = load_config(
                    &file_system,
                    PathBuf::from("config").join("campaign_defs.txt"),
                )
                .unwrap();

                // println!("campaign_defs: {:#?}", _campaign_defs);

                let scene = Box::new(
                    MainMenuScene::new(file_system, context.clone(), &surface_desc).unwrap(),
                );

                tracing::info!("Application initialized!");

                *self = App::Initialized {
                    window,
                    surface,
                    surface_desc,
                    context,
                    #[cfg(feature = "egui")]
                    egui_integration,
                    frame_index: 0,
                    last_frame_time: Instant::now(),
                    scene,
                    requested_scene: None,
                    _main_thread_receiver: main_thread_receiver.clone(),
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
                context,
                frame_index,
                last_frame_time,
                #[cfg(feature = "egui")]
                egui_integration,
                scene,
                requested_scene,
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

                        surface.resize(&context.device, size);
                        surface_desc.size = surface.size();

                        scene.resize(size);

                        window.request_redraw();
                    }

                    WindowEvent::RedrawRequested => {
                        // If a new scene was requested, switch to it before processing the frame.
                        if let Some(requested_scene) = requested_scene.take() {
                            *scene = requested_scene;
                            scene.resize(surface.size());
                        }

                        let now = Instant::now();
                        let last_frame_duration = now - *last_frame_time;
                        *last_frame_time = now;

                        {
                            let delta_time = last_frame_duration.as_secs_f32();
                            scene.update(delta_time);
                        }

                        {
                            let output = surface.get_texture(&context.device);
                            let surface_view = output
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());

                            let encoder = context.device.create_command_encoder(
                                &wgpu::CommandEncoderDescriptor {
                                    label: Some("main command encoder"),
                                },
                            );

                            let mut frame = Frame {
                                encoder,
                                surface: surface_view,
                                frame_index: *frame_index,
                                size: surface.size(),
                            };

                            scene.render(context, &mut frame);

                            // Render egui if it requires a repaint.
                            #[cfg(feature = "egui")]
                            if repaint {
                                egui_integration.render(
                                    window,
                                    &mut frame.encoder,
                                    &frame.surface,
                                    |ctx| {
                                        ctx.set_pixels_per_point(1.2);
                                        // Debug stuff from the scene.
                                        scene.debug_panel(ctx, frame.frame_index);
                                    },
                                );
                            }

                            context
                                .queue
                                .submit(std::iter::once(frame.encoder.finish()));

                            output.present();

                            // Frame is done rendering.

                            *frame_index += 1;

                            window.request_redraw();
                        }
                    }

                    _ => {}
                }

                if let Some(input_event) = input::translate_window_event(&event) {
                    scene.input_event(&input_event);
                }
            }
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: MainThreadEvent) {
        let Self::Initialized { scene, surface, .. } = self else {
            return;
        };

        match event {
            MainThreadEvent::ReplaceScene(new_scene) => {
                *scene = new_scene;
                scene.resize(surface.size());
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

    let event_loop = EventLoop::<MainThreadEvent>::with_user_event()
        .build()
        .unwrap();
    let main_thread_receiver = MainThreadReceiver::new(event_loop.create_proxy());

    let mut app = App::Uninitialzed(opts, main_thread_receiver);
    event_loop
        .run_app(&mut app)
        .expect("run application event loop");
}
