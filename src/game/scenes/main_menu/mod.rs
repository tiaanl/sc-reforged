use std::{path::PathBuf, sync::Arc};

use bevy_ecs::prelude::*;

use crate::{
    engine::{
        assets::AssetError,
        input::InputState,
        renderer::{Frame, RenderContext, SurfaceDesc},
        scene::Scene,
    },
    game::{
        assets::images::Images,
        config::{
            load_config,
            windows::{GeometryKind, WindowBase},
        },
        file_system::FileSystem,
    },
};

mod ecs;
mod window_renderer;

#[derive(Resource)]
struct DeltaTime(f32);

#[derive(Resource)]
struct AnimationState {
    frames: [Entity; 5],
    current_frame: usize,
    fading_out: bool,
}

#[derive(Default, Resource)]
struct RenderSnapshot {
    primitives: window_renderer::Primitives,
}

pub struct MainMenuScene {
    world: World,
    update_schedule: Schedule,

    renderer: window_renderer::WindowRenderer,
}

impl MainMenuScene {
    const FRAME_FADE_SPEED: f32 = 0.4;

    pub fn new(
        file_system: Arc<FileSystem>,
        context: &RenderContext,
        surface: &SurfaceDesc,
    ) -> Result<Self, AssetError> {
        let mut world = World::default();

        world.insert_resource(DeltaTime(0.0));
        world.insert_resource(RenderSnapshot::default());

        let window_base: WindowBase = load_config(
            file_system.as_ref(),
            PathBuf::from("config")
                .join("window_bases")
                .join("main_menu.txt"),
        )?;

        let images = Arc::new(Images::new(Arc::clone(&file_system)));

        let mut window_renderer =
            window_renderer::WindowRenderer::new(context, surface, Arc::clone(&images));

        Self::spawn_buttons(&images, &mut world, &window_base);

        let mut frames = [Entity::PLACEHOLDER; 5];

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

                    let image_handle = images.load(path)?;
                    // TODO: We should not unwrap here.
                    let texture = window_renderer
                        .create_texture(context, image_handle)
                        .unwrap();

                    let new_frame = world.spawn(ecs::geometry::GeometryTiled {
                        texture,
                        alpha: if i <= 1 { 1.0 } else { 0.0 },
                        size: glam::IVec2::from(geometry.dimensions).as_uvec2(),
                    });
                    frames[i] = new_frame.id();
                }
            }
        }

        world.insert_resource(AnimationState {
            frames,
            current_frame: 0,
            fading_out: false,
        });

        let mut update_schedule = Schedule::default();
        update_schedule.add_systems((rotate_background_alphas, update_render_snapshot).chain());

        Ok(Self {
            world,
            update_schedule,
            renderer: window_renderer,
        })
    }

    fn spawn_buttons(images: &Images, world: &mut World, window_base: &WindowBase) {
        macro_rules! get_ivar {
            ($name:literal) => {{
                window_base
                    .ivars
                    .get("button_offset_x")
                    .cloned()
                    .unwrap_or(0)
            }};
        }

        let button_offset_x = get_ivar!("button_offset_x");
        let button_offset_y = get_ivar!("button_offset_y");

        let shadow_offset_x = get_ivar!("shadow_offset_x");
        let shadow_offset_y = get_ivar!("shadow_offset_y");

        const BUTTONS: &[(&str, u32, u32)] = &[
            ("b_new_game", 325, 80),
            ("b_load_game", 320, 120),
            ("b_training", 315, 160),
            ("b_options", 310, 200),
            ("b_intro", 305, 240),
            ("b_multiplayer", 300, 280),
            ("b_exit", 295, 320),
        ];

        for (id, x, y) in BUTTONS {
            world.spawn((
                ecs::Widget {
                    position: glam::UVec2::new(*x, *y),
                    size: glam::UVec2::new(100, 100),
                },
                // ecs::WidgetRenderer { texture: todo!() },
            ));
        }
    }
}

impl Scene for MainMenuScene {
    fn resize(&mut self, size: glam::UVec2) {
        self.renderer.resize(size);
    }

    fn update(&mut self, delta_time: f32, _input: &InputState) {
        self.world.resource_mut::<DeltaTime>().0 = delta_time;
        self.update_schedule.run(&mut self.world);
    }

    fn render(&mut self, context: &RenderContext, frame: &mut Frame) {
        let snapshot = self.world.resource::<RenderSnapshot>();
        self.renderer.submit(context, frame, &snapshot.primitives);
    }
}

fn rotate_background_alphas(
    mut state: ResMut<AnimationState>,
    mut geometries: Query<&mut ecs::geometry::GeometryTiled>,
    time: Res<DeltaTime>,
) {
    let Ok(mut frames) = geometries.get_many_mut(state.frames) else {
        return;
    };

    let current_frame = state.current_frame.min(frames.len().saturating_sub(1));
    state.current_frame = current_frame;

    let step = time.0.max(0.0) * MainMenuScene::FRAME_FADE_SPEED;
    let max_fade_out_frame = frames.len().saturating_sub(2);

    if state.fading_out {
        {
            let frame = &mut frames[current_frame];
            frame.alpha = (frame.alpha - step).max(0.0);
        }

        if frames[current_frame].alpha <= 0.0 {
            if current_frame == max_fade_out_frame {
                state.fading_out = false;
            } else {
                state.current_frame += 1;

                if let Some(next_frame) = frames.get_mut(current_frame + 2) {
                    next_frame.alpha = 1.0;
                }
            }
        }
    } else {
        {
            let frame = &mut frames[current_frame];
            frame.alpha = (frame.alpha + step).min(1.0);
        }

        if frames[current_frame].alpha >= 1.0 {
            if current_frame == 0 {
                state.fading_out = true;
            } else {
                state.current_frame -= 1;
                frames[current_frame + 1].alpha = 0.0;
            }
        }
    }
}

fn update_render_snapshot(
    state: Res<AnimationState>,
    frames: Query<&ecs::geometry::GeometryTiled>,
    mut snapshot: ResMut<RenderSnapshot>,
) {
    snapshot.primitives.clear();

    for frame in frames.iter_many(state.frames).rev() {
        snapshot.primitives.add_rect(
            glam::Vec2::ZERO,
            frame.size.as_vec2(),
            frame.texture,
            frame.alpha,
        );
    }
}
