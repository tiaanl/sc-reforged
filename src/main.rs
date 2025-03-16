use std::{path::PathBuf, sync::Arc, time::Instant};

use clap::Parser;
use egui::Widget;
use engine::{egui_integration::EguiIntegration, prelude::*};
use game::{
    asset_loader::AssetLoader,
    config,
    scenes::{model_viewer::ModelViewer, world::WorldScene},
};
use tracing::{error, info, warn};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, KeyEvent},
    keyboard::PhysicalKey,
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
        egui_integration: engine::egui_integration::EguiIntegration,
        /// The main way of loading assets from the /data directory.
        _assets: Arc<AssetLoader>,
        _asset_store: AssetStore,

        input: InputState,

        /// The last position the mouse was on the window client area.
        last_mouse_position: Option<Vec2>,

        // The instant that the last frame started to render.
        last_frame_time: Instant,
        /// The scene we are currently rendering to the screen.
        scene: Box<dyn Scene>,
    },
}

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        match self {
            App::Uninitialzed(opts) => {
                event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

                let screen_size = event_loop
                    .primary_monitor()
                    .expect("get primary monitor info")
                    .size();

                let position = winit::dpi::Position::Physical(PhysicalPosition::new(
                    screen_size.width as i32 / 4,
                    screen_size.height as i32 / 4,
                ));

                let attributes = winit::window::WindowAttributes::default()
                    .with_title("Shadow Company - Reforged")
                    // .with_inner_size(winit::dpi::LogicalSize::new(640, 480))
                    .with_inner_size(winit::dpi::LogicalSize::new(1280, 800))
                    .with_position(position)
                    // .with_resizable(false)
                    // .with_enabled_buttons(
                    //     winit::window::WindowButtons::MINIMIZE
                    //         | winit::window::WindowButtons::CLOSE,
                    // )
                    //.with_system_backdrop(winit::platform::windows::BackdropType::None)
                    ;
                let window = Arc::new(
                    event_loop
                        .create_window(attributes)
                        .expect("create main window"),
                );

                let renderer = Renderer::new(Arc::clone(&window));

                let egui_integration = EguiIntegration::new(event_loop, &renderer);

                let asset_store = AssetStore::default();
                let asset_loader = Arc::new(
                    AssetLoader::new(asset_store.clone(), &opts.path)
                        .expect("Could not initialize assets."),
                );

                let scene: Box<dyn Scene> = if true {
                    // WorldScene

                    let campaign_defs = asset_loader
                        .load_config::<config::CampaignDefs>(
                            &PathBuf::from("config").join("campaign_defs.txt"),
                        )
                        .unwrap();

                    // Campaigns and total texture count.

                    let campaign_def = campaign_defs
                        .campaigns
                        .iter()
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

                    Box::new(
                        match WorldScene::new(
                            &asset_loader,
                            asset_store.clone(),
                            &renderer,
                            campaign_def,
                        ) {
                            Ok(scene) => scene,
                            Err(err) => {
                                error!("Could not create world scene! - {}", err);
                                panic!();
                            }
                        },
                    )
                } else {
                    Box::new(ModelViewer::new(&renderer, Arc::clone(&asset_loader)).unwrap())
                };

                info!("Application initialized!");

                *self = App::Initialized {
                    window,
                    renderer,
                    egui_integration,
                    _assets: asset_loader,
                    _asset_store: asset_store,
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

                let egui_winit::EventResponse { consumed, repaint } =
                    egui_integration.window_event(window.as_ref(), &event);
                if consumed {
                    return;
                }

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

                        let output = renderer.surface.get_current_texture().unwrap();
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
                        };

                        {
                            scene.render(&mut frame);
                            input.reset_current_frame();
                        }

                        // Render egui if it requires a repaint.
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

                        let event = if state.is_pressed() {
                            SceneEvent::MouseDown { position, button }
                        } else {
                            SceneEvent::MouseUp { position, button }
                        };

                        scene.event(&event);
                    }

                    WindowEvent::CursorMoved {
                        position: PhysicalPosition { x, y },
                        ..
                    } => {
                        *last_mouse_position = Some(Vec2::new(x as f32, y as f32));
                    }

                    WindowEvent::CursorLeft { .. } => {
                        *last_mouse_position = None;
                    }

                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                physical_key: PhysicalKey::Code(code),
                                state,
                                ..
                            },
                        ..
                    } => scene.event(&match state {
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

    let event_loop = winit::event_loop::EventLoop::new().unwrap();

    let mut app = App::Uninitialzed(opts);
    event_loop
        .run_app(&mut app)
        .expect("run application event loop");
}
