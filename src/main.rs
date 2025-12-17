use std::{path::PathBuf, sync::Arc, time::Instant};

use clap::Parser;

use glam::UVec2;
use winit::dpi::PhysicalPosition;

use crate::{
    engine::{
        context::{EngineContext, EngineEvent},
        input::InputState,
        renderer::{Frame, Renderer, Surface},
        scene::{Scene, SceneLoader},
    },
    game::{
        data_dir::{DataDir, scoped_data_dir},
        file_system::scoped_file_system,
        image::{Images, scoped_images},
        models::{Models, scoped_models},
        scenes::select_campaign::SelectCampaignSceneLoader,
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
    Uninitialzed(Opts, winit::event_loop::EventLoopProxy<EngineEvent>),
    Initialized {
        /// Hold a copy of our own context to give out to others.
        engine_context: EngineContext,
        /// The main window the engine is rendering to. This is also the window
        /// that is receiving all the input events.
        window: Arc<winit::window::Window>,
        /// The window surface where the scene will be displayed.
        surface: Surface,
        /// The placeholder for the scoped global [Renderer].
        renderer: Renderer,
        /// The current input state of the engine.
        input: InputState,
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
    },
}

impl winit::application::ApplicationHandler<EngineEvent> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        match self {
            App::Uninitialzed(_opts, event_loop_proxy) => {
                event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

                let mut attributes = winit::window::WindowAttributes::default()
                    .with_title("Shadow Company - Reforged")
                    // .with_inner_size(winit::dpi::LogicalSize::new(640, 480))
                    .with_inner_size(winit::dpi::LogicalSize::new(1600.0, 900.0));

                // No resizing?
                if false {
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

                let (surface, renderer) = engine::renderer::create(Arc::clone(&window));

                #[cfg(feature = "egui")]
                let egui_integration = engine::egui_integration::EguiIntegration::new(
                    event_loop,
                    renderer.device.clone(),
                    renderer.queue.clone(),
                    surface.format(),
                );

                let engine_context = EngineContext::new(event_loop_proxy.clone());

                let scene = SelectCampaignSceneLoader::load_scene(
                    Box::new(SelectCampaignSceneLoader),
                    engine_context.clone(),
                    &renderer,
                    surface.format(),
                )
                .unwrap();

                tracing::info!("Application initialized!");

                *self = App::Initialized {
                    engine_context: engine_context.clone(),
                    window,
                    surface,
                    renderer,
                    #[cfg(feature = "egui")]
                    egui_integration,
                    input: InputState::default(),
                    frame_index: 0,
                    last_frame_time: Instant::now(),
                    scene,
                    requested_scene: None,
                };
            }

            App::Initialized { .. } => {
                tracing::warn!("Application already initialized!");
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        use winit::event::WindowEvent;
        match self {
            App::Uninitialzed(..) => {
                tracing::warn!("Can't process events for uninitialized application.");
            }
            App::Initialized {
                engine_context,
                window,
                surface,
                renderer,
                input,
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

                        surface.resize(&renderer.device, size);

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
                            scene.update(delta_time, input);
                        }

                        {
                            let output = surface.get_texture(&renderer.device);
                            let surface_view = output
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());

                            let encoder = renderer.device.create_command_encoder(
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

                            {
                                scene.render(renderer, &mut frame);
                                input.reset_current_frame();
                            }

                            // Render egui if it requires a repaint.
                            #[cfg(feature = "egui")]
                            if repaint {
                                egui_integration.render(
                                    window,
                                    &mut frame.encoder,
                                    &frame.surface,
                                    |ctx| {
                                        ctx.set_pixels_per_point(1.2);

                                        egui::Window::new("Engine").show(ctx, |ui| {
                                            if ui.button("Loading").clicked() {
                                                use crate::game::scenes::select_campaign::SelectCampaignSceneLoader;
                                                engine_context.switch_scene(SelectCampaignSceneLoader);
                                            }
                                        });

                                        // Debug stuff from the scene.
                                        scene.debug_panel(ctx, frame.frame_index);
                                    },
                                );
                            }

                            renderer
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

                input.handle_window_event(event);
            }
        }
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: EngineEvent) {
        let App::Initialized {
            engine_context,
            surface,
            renderer,
            requested_scene,
            ..
        } = self
        else {
            return;
        };

        match event {
            EngineEvent::_Exit => {
                tracing::info!("Exit requested!");
                event_loop.exit();
            }
            EngineEvent::LoadScene(loader) => {
                let engine_context = engine_context.clone();
                let renderer = renderer.clone();
                let surface_format = surface.format();

                std::thread::spawn(move || {
                    let scene = loader
                        .load_scene(engine_context.clone(), &renderer, surface_format)
                        .unwrap();
                    engine_context
                        .event_loop_proxy
                        .send_event(EngineEvent::SwitchScene(scene))
                });
            }
            EngineEvent::SwitchScene(new_scene) => {
                *requested_scene = Some(new_scene);
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

    let _file_system = scoped_file_system(|| game::file_system::FileSystem::new(opts.path.clone()));
    let _data_dir = scoped_data_dir(|| DataDir);
    let _images = scoped_images(|| Images::new().expect("Could not initialize images."));
    let _models = scoped_models(|| Models::new().expect("Could not initialize models."));

    let event_loop: winit::event_loop::EventLoop<EngineEvent> =
        winit::event_loop::EventLoop::with_user_event()
            .build()
            .unwrap();

    let mut app = App::Uninitialzed(opts, event_loop.create_proxy());
    event_loop
        .run_app(&mut app)
        .expect("run application event loop");
}
