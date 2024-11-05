use std::{path::PathBuf, sync::Arc, time::Instant};

use clap::Parser;
use egui::Widget;
use engine::{assets::Assets, egui_integration::EguiIntegration, renderer::Renderer, scene::Scene};
use game::{
    config::{read_compaign_defs, CampaignDef},
    scenes::world::WorldScene,
};
use tracing::{error, info, warn};

mod engine;
mod game;

#[derive(clap::Parser)]
struct Opts {
    /// Path to the game data directory.
    /// (e.g. "C:\Program Files\Sinister Games\Shadow Comapany - Left for Dead\Data")
    path: PathBuf,
}

enum App {
    Uninitialzed(Opts),
    Initialized {
        window: Arc<winit::window::Window>,

        /// The renderer.
        renderer: Renderer,

        egui_integration: engine::egui_integration::EguiIntegration,

        _assets: Assets,

        // The instant that the last frame started to render.
        last_frame_time: Instant,

        // All the available campaign definitions.
        _campaign_defs: Vec<CampaignDef>,

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

                let position = winit::dpi::Position::Physical(winit::dpi::PhysicalPosition::new(
                    screen_size.width as i32 / 4,
                    screen_size.height as i32 / 4,
                ));

                let attributes = winit::window::WindowAttributes::default()
                    .with_title("Shadow Company - Left for Dead (granite)")
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

                let assets = Assets::new(&opts.path);

                let s = assets.load_config_file("config/campaign_defs.txt").unwrap();
                let campaign_defs = read_compaign_defs(&s);
                let campaign_def = campaign_defs
                    .iter()
                    .find(|c| c.base_name == "training")
                    .cloned()
                    .unwrap();

                let scene: Box<dyn Scene> = if false {
                    use game::scenes::loading::LoadingScene;
                    Box::new(LoadingScene::new(&assets, &renderer))
                } else {
                    Box::new(match WorldScene::new(&assets, &renderer, campaign_def) {
                        Ok(scene) => scene,
                        Err(err) => {
                            error!("Could not create world scene! - {}", err);
                            panic!();
                        }
                    })
                };

                info!("Application initialized!");

                *self = App::Initialized {
                    window,
                    renderer,
                    egui_integration,
                    _assets: assets,
                    last_frame_time: Instant::now(),
                    _campaign_defs: campaign_defs,
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
                        scene.resize(width, height);

                        window.request_redraw();
                    }

                    WindowEvent::KeyboardInput { event, .. } => {
                        if let winit::keyboard::PhysicalKey::Code(key) = event.physical_key {
                            if !event.repeat {
                                if event.state == winit::event::ElementState::Pressed {
                                    scene.on_key_pressed(key);
                                } else {
                                    scene.on_key_released(key);
                                }
                            }
                        }
                    }

                    WindowEvent::MouseInput { state, button, .. } => {
                        if state == winit::event::ElementState::Pressed {
                            scene.on_mouse_pressed(button);
                        } else {
                            scene.on_mouse_released(button);
                        }
                    }

                    WindowEvent::CursorMoved { position, .. } => {
                        scene.on_mouse_moved(glam::vec2(position.x as f32, position.y as f32));
                    }

                    WindowEvent::RedrawRequested => {
                        let now = Instant::now();
                        let last_frame_duration = now - *last_frame_time;
                        *last_frame_time = now;

                        let output = renderer.surface.get_current_texture().unwrap();
                        let view = output
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());

                        let mut encoder = renderer.device.create_command_encoder(
                            &wgpu::CommandEncoderDescriptor {
                                label: Some("main command encoder"),
                            },
                        );

                        {
                            scene.update(last_frame_duration.as_secs_f32() * 60.0);

                            scene.begin_frame();
                            scene.render(&renderer, &mut encoder, &view);
                            scene.end_frame();
                        }

                        // Render egui if it requires a repaint.
                        if repaint {
                            egui_integration.render(window, renderer, &mut encoder, &view, |ctx| {
                                egui::Area::new(egui::Id::new("engine_info")).show(ctx, |ui| {
                                    let fps_label = {
                                        let text = egui::WidgetText::RichText(egui::RichText::new(
                                            format!(
                                                "{:0.1}",
                                                1.0 / last_frame_duration.as_secs_f64()
                                            ),
                                        ))
                                        .background_color(epaint::Color32::from_rgba_premultiplied(
                                            0, 0, 0, 127,
                                        ))
                                        .monospace();
                                        egui::Label::new(text.color(epaint::Color32::WHITE))
                                            .wrap_mode(egui::TextWrapMode::Extend)
                                    };

                                    fps_label.ui(ui);
                                });

                                // Debug stuff from the scene.
                                scene.debug_panel(ctx);
                            });
                        }

                        renderer.queue.submit(std::iter::once(encoder.finish()));

                        output.present();

                        window.request_redraw();
                    }

                    _ => {}
                }
            }
        }
    }
}

fn main() {
    tracing_subscriber::fmt().init();

    let opts = Opts::parse();

    let event_loop = winit::event_loop::EventLoop::new().unwrap();

    let mut app = App::Uninitialzed(opts);
    event_loop
        .run_app(&mut app)
        .expect("run application event loop");
}
