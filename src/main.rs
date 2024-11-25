use std::{path::PathBuf, sync::Arc, time::Instant};

use clap::Parser;
use egui::Widget;
use engine::{
    assets::AssetLoader, egui_integration::EguiIntegration, input, renderer::Renderer, scene::Scene,
};
use game::{
    config::{read_compaign_defs, CampaignDef},
    scenes::{model_viewer::ModelViewer, world::WorldScene},
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
        /// The main window the engine is rendering to. This is also the window
        /// that is receiving all the input events.
        window: Arc<winit::window::Window>,
        /// The renderer.
        renderer: Renderer,
        /// egui integration.
        egui_integration: engine::egui_integration::EguiIntegration,
        /// The main way of loading assets from the /data directory.
        _assets: AssetLoader,

        input: input::InputState,

        // The instant that the last frame started to render.
        last_frame_time: Instant,
        /// The scene we are currently rendering to the screen.
        scene: Box<dyn Scene>,

        /// All the available campaign definitions.
        /// This is temporary at the moment, and should go into the main menu
        /// scene.
        _campaign_defs: Vec<CampaignDef>,
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

                let assets = AssetLoader::new(&opts.path);

                let s = assets.load_config_file("config/campaign_defs.txt").unwrap();
                let campaign_defs = read_compaign_defs(&s);
                let campaign_def = campaign_defs
                    .iter()
                    .find(|c| c.base_name == "training")
                    .cloned()
                    .unwrap();

                let scene: Box<dyn Scene> = if false {
                    // LoadingScene

                    use game::scenes::loading::LoadingScene;
                    Box::new(LoadingScene::new(&assets, &renderer))
                } else if true {
                    // WorldScene

                    Box::new(match WorldScene::new(&assets, &renderer, campaign_def) {
                        Ok(scene) => scene,
                        Err(err) => {
                            error!("Could not create world scene! - {}", err);
                            panic!();
                        }
                    })
                } else {
                    // ModelViewer

                    Box::new(
                        match ModelViewer::new(
                            &assets,
                            &renderer,
                            r"models\alvhqd-hummer\alvhqd-hummer.smf",
                            // r"models\AlVhAp-Cessna\AlVhAp-Cessna.smf",
                            // r"models\agsths-metalshack\agsths-metalshack.smf",
                        ) {
                            Ok(scene) => scene,
                            Err(err) => {
                                error!("Could not create model viewer scene! - {}", err);
                                panic!();
                            }
                        },
                    )
                };

                info!("Application initialized!");

                *self = App::Initialized {
                    window,
                    renderer,
                    egui_integration,
                    _assets: assets,
                    input: input::InputState::default(),
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
                input,
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
                            let delta_time = last_frame_duration.as_secs_f32() * 60.0;

                            scene.on_input(input, delta_time);

                            scene.update(delta_time);

                            scene.begin_frame();
                            scene.render(&renderer, &mut encoder, &view);
                            scene.end_frame();

                            input.reset_current_frame();
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

                    _ => input.handle_window_event(event),
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
