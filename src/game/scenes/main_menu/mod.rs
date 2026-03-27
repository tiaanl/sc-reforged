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
        world.init_resource::<Messages<ecs::WindowMessage>>();

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
        update_schedule.add_systems(
            (
                update_button_hover_state,
                animate_button_shadow,
                rotate_background_alphas,
                update_render_snapshot,
            )
                .chain(),
        );

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
        const BASE_TEXT_ALPHA: f32 = 200.0 / 255.0;

        let sprite_frame_size = |sprite_name: &str, frame: usize| {
            let sprite = sprites.get_handle_by_name(sprite_name)?;
            let frame_data = sprites
                .get(sprite)
                .and_then(|sprite_data| sprite_data.frame(frame))?;

            Some(frame_data.bottom_right - frame_data.top_left)
        };

        let spawn_sprite = |world: &mut World,
                            position: glam::IVec2,
                            sprite_name: &str,
                            frame: usize,
                            alpha: f32| {
            let sprite = sprites.get_handle_by_name(sprite_name)?;
            let frame_size = sprite_frame_size(sprite_name, frame)?;

            let entity = world
                .spawn((ecs::SpriteRender {
                    position: position.as_vec2(),
                    alpha,
                    sprite,
                    frame,
                },))
                .id();

            Some((entity, frame_size))
        };

        for button in BUTTONS {
            let base_position = window_base
                .button_advices
                .get(button.name)
                .map(|button| glam::IVec2::new(button.x, button.y))
                .unwrap_or(glam::IVec2::ZERO);

            let Some((shadow_entity, _)) = spawn_sprite(
                world,
                base_position + shadow_offset,
                button.shadow_sprite,
                button.shadow_frame,
                BASE_TEXT_ALPHA,
            ) else {
                continue;
            };
            let Some(bullet_size) = sprite_frame_size(BULLET_SPRITE, BULLET_FRAME) else {
                continue;
            };
            let Some((text_entity, text_size)) = spawn_sprite(
                world,
                base_position + button_offset,
                button.text_sprite,
                button.text_frame,
                BASE_TEXT_ALPHA,
            ) else {
                continue;
            };

            world.spawn((
                ecs::Widget {
                    position: base_position.as_vec2(),
                    size: glam::UVec2::new(bullet_size.x + text_size.x, text_size.y),
                },
                ecs::MainMenuButtonAnimation {
                    button_offset,
                    shadow_offset,
                    shadow_entity,
                    text_entity,
                    hover_progress_ms: 0.0,
                    hovered: false,
                },
            ));
        }
    }
}

impl Scene for MainMenuScene {
    fn resize(&mut self, size: glam::UVec2) {
        self.window_renderer.resize(size);
    }

    fn update(&mut self, delta_time: f32, input: &InputState) {
        self.world.resource_mut::<DeltaTime>().0 = delta_time;

        if let Some(mouse_position) = input.mouse_position() {
            self.world
                .write_message(ecs::WindowMessage::MouseMove(mouse_position));
        } else {
            self.world.write_message(ecs::WindowMessage::MouseLeave);
        }

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

fn update_button_hover_state(
    mut messages: MessageReader<ecs::WindowMessage>,
    mut buttons: Query<(&ecs::Widget, &mut ecs::MainMenuButtonAnimation)>,
) {
    let mut mouse_position = None;
    let mut has_input_update = false;

    for message in messages.read() {
        has_input_update = true;

        match message {
            ecs::WindowMessage::MouseMove(position) => mouse_position = Some(*position),
            ecs::WindowMessage::MouseLeave => mouse_position = None,
        }
    }

    if !has_input_update {
        return;
    }

    for (widget, mut animation) in buttons.iter_mut() {
        animation.hovered = mouse_position.is_some_and(|mouse_position| {
            let mouse_position = mouse_position.as_vec2();
            let min = widget.position;
            let max = min + widget.size.as_vec2();

            mouse_position.x >= min.x
                && mouse_position.y >= min.y
                && mouse_position.x < max.x
                && mouse_position.y < max.y
        });
    }
}

fn animate_button_shadow(
    time: Res<DeltaTime>,
    mut buttons: Query<(&ecs::Widget, &mut ecs::MainMenuButtonAnimation)>,
    mut renders: Query<&mut ecs::SpriteRender>,
) {
    const HOVER_PROGRESS_MAX_MS: f32 = 250.0;
    const HOVER_EXIT_RATE: f32 = 1.0 / 3.0;
    const SHADOW_SLIDE_SCALE: f32 = 0.004;
    const BASE_ALPHA: f32 = 200.0;
    const TEXT_ALPHA_SCALE: f32 = 0.22;
    const SHADOW_ALPHA_SCALE: f32 = 0.8;

    let delta_ms = time.0.max(0.0) * 1000.0;

    for (widget, mut animation) in buttons.iter_mut() {
        if animation.hovered {
            animation.hover_progress_ms =
                (animation.hover_progress_ms + delta_ms).min(HOVER_PROGRESS_MAX_MS);
        } else {
            animation.hover_progress_ms =
                (animation.hover_progress_ms - delta_ms * HOVER_EXIT_RATE).max(0.0);
        }

        let slide_delta =
            (HOVER_PROGRESS_MAX_MS - animation.hover_progress_ms) * SHADOW_SLIDE_SCALE;
        let text_position = widget.position + animation.button_offset.as_vec2();
        let shadow_delta =
            (animation.shadow_offset - animation.button_offset).as_vec2() * slide_delta;
        let text_alpha =
            (BASE_ALPHA + animation.hover_progress_ms * TEXT_ALPHA_SCALE).round() / 255.0;
        let shadow_alpha =
            (BASE_ALPHA - animation.hover_progress_ms * SHADOW_ALPHA_SCALE).round() / 255.0;

        if let Ok(mut shadow_render) = renders.get_mut(animation.shadow_entity) {
            shadow_render.position = text_position + shadow_delta;
            shadow_render.alpha = shadow_alpha;
        }

        if let Ok(mut text_render) = renders.get_mut(animation.text_entity) {
            text_render.alpha = text_alpha;
        }
    }
}

fn update_render_snapshot(
    state: Res<AnimationState>,
    renders: Query<&ecs::SpriteRender>,
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
    for render in renders.iter() {
        snapshot.render_items.render_sprite(
            render.position,
            render.sprite,
            render.frame,
            render.alpha,
        )
    }
}

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
