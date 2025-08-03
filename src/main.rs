use std::{path::PathBuf, sync::Arc, time::Instant};

use clap::Parser;
use engine::prelude::*;
use game::scenes::world::WorldScene;
use glam::UVec2;
use tracing::{error, info, warn};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, KeyEvent},
    keyboard::PhysicalKey,
};

use crate::{
    engine::ui,
    game::{
        data_dir::{DataDir, data_dir, scoped_data_dir},
        file_system::scoped_file_system,
        models::{Models, scoped_models},
        scenes::ui_test::UiTestScene,
    },
};

mod engine;
mod game;

#[derive(clap::Parser)]
struct Opts {
    /// Path to the game data directory.
    /// (e.g. "C:\Program Files\Sinister Games\Shadow Comapany - Left for Dead\Data")
    path: PathBuf,
}

#[allow(clippy::large_enum_variant)]
enum App {
    Uninitialzed(Opts),
    Initialized {
        /// The main window the engine is rendering to. This is also the window
        /// that is receiving all the input events.
        window: Arc<winit::window::Window>,
        /// The renderer.
        renderer: Renderer,
        /// egui integration.
        #[cfg(feature = "egui")]
        egui_integration: engine::egui_integration::EguiIntegration,
        /// The current input state of the engine.
        input: InputState,
        /// The last position the mouse was on the window client area.
        last_mouse_position: Option<UVec2>,
        // The instant that the last frame started to render.
        last_frame_time: Instant,
        /// The scene we are currently rendering to the screen.
        scene: Box<dyn Scene>,
    },
}

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        match self {
            App::Uninitialzed(_opts) => {
                event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

                let mut attributes = winit::window::WindowAttributes::default()
                    .with_title("Shadow Company - Reforged")
                    // .with_inner_size(winit::dpi::LogicalSize::new(640, 480))
                    .with_inner_size(winit::dpi::LogicalSize::new(1280, 800));

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
                let inner_size = window.inner_size();

                let renderer = Renderer::new(Arc::clone(&window));

                #[cfg(feature = "egui")]
                let egui_integration =
                    engine::egui_integration::EguiIntegration::new(event_loop, &renderer);

                let scene: Box<dyn Scene> = if true {
                    // WorldScene

                    let campaign_defs = data_dir().load_campaign_defs().unwrap();

                    // Campaigns and total texture count.

                    let campaign_def = campaign_defs
                        .campaigns
                        .iter()
                        // .find(|c| c.base_name == "test")
                        .find(|c| c.base_name == "training") // 140
                        // .find(|c| c.base_name == "angola_tutorial") // 149
                        // .find(|c| c.base_name == "angola") // 368
                        // .find(|c| c.base_name == "romania") // 289
                        // .find(|c| c.base_name == "kola") // 213
                        // .find(|c| c.base_name == "caribbean") // 279
                        // .find(|c| c.base_name == "kola_2") // 240
                        // .find(|c| c.base_name == "ecuador") // 341
                        // .find(|c| c.base_name == "peru") // 197
                        // .find(|c| c.base_name == "angola_2") // 347
                        .cloned()
                        .unwrap();

                    Box::new(match WorldScene::new(&renderer, campaign_def) {
                        Ok(scene) => scene,
                        Err(err) => {
                            error!("Could not create world scene! - {}", err);
                            panic!();
                        }
                    })
                } else {
                    Box::new(UiTestScene::new(
                        &renderer,
                        ui::Size {
                            width: inner_size.width as i32,
                            height: inner_size.height as i32,
                        },
                    ))
                };
                //  else {
                //     Box::new(ModelViewer::new(&renderer, data_dir).unwrap())
                // };

                info!("Application initialized!");

                *self = App::Initialized {
                    window,
                    renderer,
                    #[cfg(feature = "egui")]
                    egui_integration,
                    input: InputState::default(),
                    last_mouse_position: None,
                    last_frame_time: Instant::now(),
                    scene,
                };
            }

            App::Initialized { .. } => {
                warn!("Application already initialized!");
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
            App::Uninitialzed(_) => {
                warn!("Can't process events for uninitialized application.");
            }
            App::Initialized {
                window,
                renderer,
                #[cfg(feature = "egui")]
                egui_integration,
                input,
                last_mouse_position,
                last_frame_time,
                scene,
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
                        renderer.resize(width, height);
                        scene.resize(renderer);

                        window.request_redraw();
                    }

                    WindowEvent::RedrawRequested => {
                        let now = Instant::now();
                        let last_frame_duration = now - *last_frame_time;
                        *last_frame_time = now;

                        let delta_time = last_frame_duration.as_secs_f32() * 60.0;
                        scene.update(renderer, delta_time, input);

                        let output = renderer.surface.get_texture();
                        let surface = output
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());

                        let encoder = renderer.device.create_command_encoder(
                            &wgpu::CommandEncoderDescriptor {
                                label: Some("main command encoder"),
                            },
                        );

                        let mut frame = Frame {
                            device: renderer.device.clone(),
                            queue: renderer.queue.clone(),
                            depth_buffer: renderer.depth_buffer.clone(),
                            encoder,
                            surface,
                            renderer,
                        };

                        {
                            scene.render(&mut frame);
                            input.reset_current_frame();
                        }

                        // Render egui if it requires a repaint.
                        #[cfg(feature = "egui")]
                        if repaint {
                            egui_integration.render(
                                window,
                                renderer,
                                &mut frame.encoder,
                                &frame.surface,
                                |ctx| {
                                    egui::Area::new(egui::Id::new("engine_info")).show(ctx, |ui| {
                                        let fps_label = {
                                            let text = egui::WidgetText::RichText(
                                                egui::RichText::new(format!(
                                                    "{:0.1}",
                                                    1.0 / last_frame_duration.as_secs_f64()
                                                )),
                                            )
                                            .background_color(
                                                epaint::Color32::from_rgba_premultiplied(
                                                    0, 0, 0, 127,
                                                ),
                                            )
                                            .monospace();
                                            egui::Label::new(text.color(epaint::Color32::WHITE))
                                                .wrap_mode(egui::TextWrapMode::Extend)
                                        };

                                        use egui::Widget;

                                        fps_label.ui(ui);
                                    });

                                    // Debug stuff from the scene.
                                    scene.debug_panel(ctx, renderer);
                                },
                            );
                        }

                        renderer
                            .queue
                            .submit(std::iter::once(frame.encoder.finish()));

                        output.present();

                        window.request_redraw();
                    }

                    WindowEvent::MouseInput { button, state, .. } => {
                        let position =
                            last_mouse_position.expect("mouse button without a position?");

                        scene.event(if state.is_pressed() {
                            SceneEvent::MouseDown { position, button }
                        } else {
                            SceneEvent::MouseUp { position, button }
                        });
                    }

                    WindowEvent::CursorMoved {
                        position: PhysicalPosition { x, y },
                        ..
                    } => {
                        let position = UVec2::new(x as u32, y as u32);
                        *last_mouse_position = Some(position);
                        scene.event(SceneEvent::MouseMove { position });
                    }

                    WindowEvent::CursorLeft { .. } => {
                        *last_mouse_position = None;
                        scene.event(SceneEvent::MouseLeft);
                    }

                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                physical_key: PhysicalKey::Code(code),
                                state,
                                ..
                            },
                        ..
                    } => scene.event(match state {
                        ElementState::Pressed => SceneEvent::KeyDown { key: code },
                        ElementState::Released => SceneEvent::KeyUp { key: code },
                    }),

                    _ => {}
                }

                input.handle_window_event(event);
            }
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        match self {
            App::Uninitialzed(_) => {
                warn!("Can't process events for uninitialized application.");
            }
            App::Initialized { input, .. } => {
                input.handle_device_event(event);
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
    let _data_dir = scoped_data_dir(DataDir::new);
    let _models = scoped_models(|| Models::new().expect("Could not initialize models."));

    let event_loop = winit::event_loop::EventLoop::new().unwrap();

    let mut app = App::Uninitialzed(opts);
    event_loop
        .run_app(&mut app)
        .expect("run application event loop");
}
