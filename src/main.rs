use tracing::{info, warn};

use std::{path::PathBuf, sync::Arc, time::Instant};

use clap::Parser;
use engine::{assets::Assets, renderer::Renderer, scene::Scene, vfs::FileSystem};
use game::{
    config::{read_compaign_defs, CampaignDef},
    scenes::world::WorldScene,
};
use winit::platform::windows::WindowAttributesExtWindows;

mod engine;
mod game;

#[derive(clap::Parser)]
struct Opts {
    /// Path to the game data directory.
    /// (e.g. "C:\Program Files\Sinister Games\Shadow Comapany - Left for Dead\Data")
    path: Option<PathBuf>,
}

enum App {
    Uninitialzed(Opts),
    Initialized {
        window: Arc<winit::window::Window>,

        /// The renderer.
        renderer: Renderer,

        assets: Assets,

        // The instant that the last frame started to render.
        last_frame_time: Instant,

        // All the available campaign definitions.
        campaign_defs: Vec<CampaignDef>,

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
                    .with_inner_size(winit::dpi::LogicalSize::new(640, 480))
                    .with_position(position)
                    // .with_resizable(false)
                    .with_system_backdrop(winit::platform::windows::BackdropType::None)
                    .with_enabled_buttons(
                        winit::window::WindowButtons::MINIMIZE
                            | winit::window::WindowButtons::CLOSE,
                    );
                let window = Arc::new(
                    event_loop
                        .create_window(attributes)
                        .expect("create main window"),
                );

                let renderer = Renderer::new(Arc::clone(&window));

                let path = opts
                    .path
                    .clone()
                    .unwrap_or(PathBuf::from("C:\\Games\\shadow_company\\data"));
                let vfs = FileSystem::new(path);

                let assets = Assets::new(vfs);

                let s = assets.load_config_file("config/campaign_defs.txt").unwrap();
                let campaign_defs = read_compaign_defs(&s);
                let campaign_def = campaign_defs
                    .iter()
                    .find(|c| c.base_name == "training")
                    .cloned()
                    .unwrap();

                // let scene = Box::new(LoadingScene::new(vfs.as_ref(), &renderer));
                // let scene = Box::new(WorldScene::new(&assets, &renderer, campaign_def));

                let scene = Box::new(WorldScene::new(&assets, &renderer, campaign_def));

                info!("Application initialized!");

                *self = App::Initialized {
                    window,
                    renderer,
                    assets,
                    last_frame_time: Instant::now(),
                    campaign_defs,
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
                last_frame_time,
                scene,
                ..
            } => {
                if window_id != window.id() {
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

                        scene.update(1.0 / last_frame_duration.as_secs_f32() / 60.0);

                        scene.begin_frame();
                        scene.render(&renderer, &mut encoder, &view);
                        scene.end_frame();

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

    // let vfs = engine::vfs::VirtualFileSystem::new(opts.path);

    // let config_file = vfs.open("config/campaign_defs.txt").unwrap();

    // let _campaigns =
    //     game::config::read_compaign_defs(&String::from_utf8_lossy(config_file.as_ref()));

    // for c in _campaigns
    //     .iter()
    //     .map(|c| format!("{} ({})", c.title, c.base_name))
    // {
    //     println!("campaign: {}", c);
    // }

    // let image_defs_file = vfs.open("config/image_defs.txt").unwrap();
    // let images = game::config::read_image_defs(&String::from_utf8_lossy(image_defs_file.as_ref()));

    // println!("images: {:#?}", images);

    // let window_base_file = vfs.open("config/window_bases/main_menu.txt").unwrap();
    // let data = String::from_utf8_lossy(window_base_file.as_ref());
    // config::read_window_base_file(data.as_ref());

    let event_loop = winit::event_loop::EventLoop::new().unwrap();

    let mut app = App::Uninitialzed(opts);
    event_loop
        .run_app(&mut app)
        .expect("run application event loop");
}
