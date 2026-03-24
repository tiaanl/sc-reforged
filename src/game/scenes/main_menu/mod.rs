use std::path::PathBuf;

use glam::{UVec2, Vec2};

use crate::{
    engine::{
        assets::AssetError,
        input::InputState,
        renderer::{Frame, RenderContext, SurfaceDesc},
        scene::Scene,
    },
    game::{
        config::{
            load_config,
            windows::{GeometryKind, WindowBase},
        },
        file_system::FileSystem,
    },
};

mod render;

struct BackFrame {
    texture_id: render::TextureId,
    alpha: f32,
    size: [i32; 2],
}

struct FrameAnimation {
    current_frame: usize,
    fading_out: bool,
}

pub struct MainMenuScene {
    window_base: WindowBase,

    renderer: render::WindowRenderer,

    frames: Vec<BackFrame>,
    frame_animation: FrameAnimation,
}

impl MainMenuScene {
    const FRAME_FADE_SPEED: f32 = 0.4;

    pub fn new(
        file_system: &FileSystem,
        context: &RenderContext,
        surface: &SurfaceDesc,
    ) -> Result<Self, AssetError> {
        let window_base: WindowBase = load_config(
            file_system,
            PathBuf::from("config")
                .join("window_bases")
                .join("main_menu.txt"),
        )?;

        let mut window_renderer = render::WindowRenderer::new(context, surface);

        let mut frames = vec![];

        for (i, geometry) in window_base.geometries.iter().enumerate() {
            match geometry.kind {
                GeometryKind::Normal(_) => {
                    tracing::warn!("Only tiled geometry supported for main menu background frames");
                    continue;
                }
                GeometryKind::Tiled(ref geometry) => {
                    let path = PathBuf::from("textures")
                        .join("interface")
                        .join(&geometry.jpeg_name)
                        .with_extension("jpg");

                    let data = file_system.load(&path)?;

                    let image =
                        image::load_from_memory_with_format(&data, image::ImageFormat::Jpeg)
                            .map_err(|err| AssetError::custom(path, format!("{err}")))?;
                    let rgba = image.into_rgba8();

                    let texture_id = window_renderer.create_texture(context, rgba);

                    frames.push(BackFrame {
                        texture_id,
                        alpha: if i <= 1 { 1.0 } else { 0.0 },
                        size: geometry.dimensions,
                    });
                }
            }
        }

        Ok(Self {
            window_base,
            renderer: window_renderer,
            frames,
            frame_animation: FrameAnimation {
                current_frame: 0,
                // The original menu starts by fading out frame1 while frame2 is already visible.
                fading_out: true,
            },
        })
    }

    /// Advance the background frame cross-fade to match the original main menu timing.
    fn update_background_frames(&mut self, delta_time: f32) {
        if self.frames.len() < 2 {
            return;
        }

        let current_frame = self
            .frame_animation
            .current_frame
            .min(self.frames.len().saturating_sub(1));
        self.frame_animation.current_frame = current_frame;

        let step = delta_time.max(0.0) * Self::FRAME_FADE_SPEED;
        let max_fade_out_frame = self.frames.len().saturating_sub(2);

        if self.frame_animation.fading_out {
            {
                let frame = &mut self.frames[current_frame];
                frame.alpha = (frame.alpha - step).max(0.0);
            }

            if self.frames[current_frame].alpha <= 0.0 {
                if current_frame == max_fade_out_frame {
                    self.frame_animation.fading_out = false;
                } else {
                    self.frame_animation.current_frame += 1;

                    if let Some(next_frame) = self.frames.get_mut(current_frame + 2) {
                        next_frame.alpha = 1.0;
                    }
                }
            }
        } else {
            {
                let frame = &mut self.frames[current_frame];
                frame.alpha = (frame.alpha + step).min(1.0);
            }

            if self.frames[current_frame].alpha >= 1.0 {
                if current_frame == 0 {
                    self.frame_animation.fading_out = true;
                } else {
                    self.frame_animation.current_frame -= 1;
                    self.frames[current_frame + 1].alpha = 0.0;
                }
            }
        }
    }
}

impl Scene for MainMenuScene {
    fn resize(&mut self, size: UVec2) {
        self.renderer.resize(size);
    }

    fn update(&mut self, delta_time: f32, _input: &InputState) {
        self.update_background_frames(delta_time);
    }

    fn render(&mut self, context: &RenderContext, frame: &mut Frame) {
        let mut primitives = render::Primitives::default();

        for f in self.frames.iter().rev() {
            primitives.add_rect(
                Vec2::ZERO,
                glam::IVec2::from(f.size).as_vec2(),
                f.texture_id,
                f.alpha,
            );
        }

        self.renderer.submit(context, frame, primitives);
    }
}
