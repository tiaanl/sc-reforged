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
        assets::{images::Images, sprites::Sprites},
        config::{
            ImageDefs, load_config,
            windows::{GeometryKind, WindowBase},
        },
        file_system::FileSystem,
        render::textures::Textures,
        scenes::main_menu::window_renderer::{RenderItems, WindowRenderer},
    },
};

mod ecs;
mod quad_renderer;
mod sprite_renderer;
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
    render_items: RenderItems,
}

pub struct MainMenuScene {
    world: World,
    update_schedule: Schedule,

    window_renderer: WindowRenderer,
}

impl MainMenuScene {
    const FRAME_FADE_SPEED: f32 = 0.4;

    pub fn new(
        file_system: Arc<FileSystem>,
        render_context: RenderContext,
        surface_desc: &SurfaceDesc,
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
        let textures = Arc::new(Textures::new(render_context.clone(), Arc::clone(&images)));

        let mut sprites = Sprites::new(Arc::clone(&images));
        let image_defs: ImageDefs =
            load_config(&file_system, PathBuf::from("config").join("image_defs.txt"))?;

        sprites.load_image_defs(&image_defs);
        let sprites = Arc::new(sprites);

        let mut window_renderer = WindowRenderer::new(
            render_context.clone(),
            surface_desc,
            Arc::clone(&textures),
            Arc::clone(&sprites),
        );

        Self::spawn_buttons(&sprites, &mut world, &window_base);

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

                    let image = images.load(path)?;

                    let Some(tiled_geometry_handle) = window_renderer.create_tiled_geometry(
                        image,
                        glam::IVec2::from(geometry.dimensions).as_uvec2(),
                        glam::IVec2::from(geometry.chunk_dimensions).as_uvec2(),
                    ) else {
                        tracing::warn!("Could not create tiled geometry from image");
                        continue;
                    };

                    let new_frame = world.spawn(ecs::geometry::GeometryTiled {
                        tiled_geometry_handle,
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
            window_renderer,
        })
    }

    fn spawn_buttons(sprites: &Sprites, world: &mut World, window_base: &WindowBase) {
        macro_rules! get_ivar {
            ($name:literal) => {{ window_base.ivars.get($name).cloned().unwrap_or(0) }};
        }

        let button_offset =
            glam::IVec2::new(get_ivar!("button_offset_x"), get_ivar!("button_offset_y"));
        let shadow_offset =
            glam::IVec2::new(get_ivar!("shadow_offset_x"), get_ivar!("shadow_offset_y"));

        const BULLET_SPRITE: &str = "interface_elements_16";
        const BULLET_FRAME: usize = 3;

        struct ButtonData<'a> {
            name: &'a str,
            text_sprite: &'a str,
            text_frame: usize,
            shadow_sprite: &'a str,
            shadow_frame: usize,
            pressed_sprite: &'a str,
            pressed_frame: usize,
        }

        impl<'a> ButtonData<'a> {
            #[allow(clippy::too_many_arguments)]
            const fn new(
                name: &'a str,
                text_sprite: &'a str,
                text_frame: usize,
                shadow_sprite: &'a str,
                shadow_frame: usize,
                pressed_sprite: &'a str,
                pressed_frame: usize,
            ) -> Self {
                Self {
                    name,
                    text_sprite,
                    text_frame,
                    shadow_sprite,
                    shadow_frame,
                    pressed_sprite,
                    pressed_frame,
                }
            }
        }

        const BUTTONS: &[ButtonData<'static>] = &[
            ButtonData::new(
                "b_new_game",
                "interface_elements_14",
                0,
                "interface_elements_14",
                1,
                "interface_elements_14",
                2,
            ),
            ButtonData::new(
                "b_load_game",
                "interface_elements_13",
                0,
                "interface_elements_13",
                1,
                "interface_elements_13",
                2,
            ),
            ButtonData::new(
                "b_training",
                "interface_elements_17",
                0,
                "interface_elements_17",
                1,
                "interface_elements_17",
                2,
            ),
            ButtonData::new(
                "b_options",
                "interface_elements_15",
                0,
                "interface_elements_15",
                1,
                "interface_elements_15",
                2,
            ),
            ButtonData::new(
                "b_intro",
                "interface_elements_13",
                3,
                "interface_elements_13",
                4,
                "interface_elements_13",
                5,
            ),
            ButtonData::new(
                "b_multiplayer",
                "interface_elements_14",
                3,
                "interface_elements_14",
                4,
                "interface_elements_14",
                5,
            ),
            ButtonData::new(
                "b_exit",
                "interface_elements_15",
                3,
                "interface_elements_15",
                4,
                "interface_elements_15",
                5,
            ),
        ];

        let spawn_sprite =
            |world: &mut World, position: glam::IVec2, sprite_name: &str, frame: usize| {
                let Some(sprite) = sprites.get_handle_by_name(sprite_name) else {
                    return;
                };

                let Some(frame_data) = sprites
                    .get(sprite)
                    .and_then(|sprite_data| sprite_data.frame(frame))
                else {
                    return;
                };

                world.spawn((
                    ecs::Widget {
                        position: position.as_uvec2(),
                        size: frame_data.bottom_right - frame_data.top_left,
                    },
                    ecs::WidgetRenderer { sprite, frame },
                ));
            };

        for button in BUTTONS {
            let base_position = window_base
                .button_advices
                .get(button.name)
                .map(|button| glam::IVec2::new(button.x, button.y))
                .unwrap_or(glam::IVec2::ZERO);

            spawn_sprite(
                world,
                base_position + shadow_offset,
                button.shadow_sprite,
                button.shadow_frame,
            );
            spawn_sprite(world, base_position, BULLET_SPRITE, BULLET_FRAME);
            spawn_sprite(
                world,
                base_position + button_offset,
                button.text_sprite,
                button.text_frame,
            );
        }
    }
}

impl Scene for MainMenuScene {
    fn resize(&mut self, size: glam::UVec2) {
        self.window_renderer.resize(size);
    }

    fn update(&mut self, delta_time: f32, _input: &InputState) {
        self.world.resource_mut::<DeltaTime>().0 = delta_time;
        self.update_schedule.run(&mut self.world);
    }

    fn render(&mut self, _context: &RenderContext, frame: &mut Frame) {
        let snapshot = self.world.resource::<RenderSnapshot>();
        self.window_renderer
            .submit_render_items(frame, &snapshot.render_items);
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
    widgets: Query<(&ecs::Widget, &ecs::WidgetRenderer)>,
    frames: Query<&ecs::geometry::GeometryTiled>,
    mut snapshot: ResMut<RenderSnapshot>,
) {
    snapshot.render_items.clear();

    // Render the background geometries.
    for frame in frames.iter_many(state.frames).rev() {
        snapshot
            .render_items
            .render_tiled_geometry(frame.tiled_geometry_handle, frame.alpha);
    }

    // Render the widgets.
    for (widget, widget_renderer) in widgets.iter() {
        snapshot.render_items.render_sprite(
            widget.position,
            widget_renderer.sprite,
            widget_renderer.frame,
        )
    }
}
