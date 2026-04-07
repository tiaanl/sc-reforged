use std::{borrow::Cow, path::PathBuf, sync::Arc};

use bevy_ecs::{prelude::*, world::CommandQueue};
use glam::{IVec2, UVec2};

use crate::{
    engine::{
        assets::AssetError,
        input::InputEvent,
        renderer::{Frame, RenderContext, SurfaceDesc},
        scene::Scene,
    },
    game::{
        assets::{images::Images, sprites::Sprites},
        config::{ImageDefs, configs::Configs, load_config, windows::WindowBase},
        file_system::FileSystem,
        render::textures::Textures,
        windows::{
            ecs::{
                WidgetMessage, WindowMessage,
                button::{Button, update_buttons},
                geometry::GeometryTiled,
                rect::{GlobalRect, Rect, update_global_rects},
                render::{SpriteRender, TextRender},
                ui_action::{UiAction, handle_ui_actions},
                widgets::Widget,
                window::{
                    Window, WindowManager, spawn_window_geometries, update_window_render_items,
                },
            },
            window_renderer::{Font, WindowRenderItems, WindowRenderer},
        },
    },
};

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
    render_items: WindowRenderItems,
}

#[derive(Component)]
pub struct MainMenuButtonAnimation {
    pub button_offset: glam::IVec2,
    pub shadow_offset: glam::IVec2,
    pub shadow_entity: Entity,
    pub text_entity: Entity,
    pub pressed_entity: Entity,
    pub hover_progress_ms: f32,
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
        world.init_resource::<WindowManager>();
        world.add_observer(
            |event: On<Add, Window>, mut window_manager: ResMut<WindowManager>| {
                window_manager.push(event.entity);
            },
        );
        world.add_observer(
            |event: On<Remove, Window>, mut window_manager: ResMut<WindowManager>| {
                window_manager.remove(event.entity);
            },
        );

        world.insert_resource(Configs::new(Arc::clone(&file_system)));

        world.init_resource::<Messages<WindowMessage>>();
        world.init_resource::<Messages<WidgetMessage>>();
        world.init_resource::<Messages<UiAction>>();

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

        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);

        let window_entity =
            crate::game::windows::ecs::window::spawn_window(&mut commands, Rect::UI).id();

        Self::spawn_buttons(&mut commands, window_entity, &sprites, &window_base);

        Self::spawn_version_label(&mut commands, &window_renderer);

        //let mut frames = [Entity::PLACEHOLDER; 5];

        let mut commands = Commands::new(&mut queue, &world);

        let frames = {
            let frames = spawn_window_geometries(
                &mut commands,
                &mut window_renderer,
                &images,
                window_entity,
                &window_base.geometries,
            )?;

            let frames =
                std::array::from_fn(|i| frames.get(i).copied().unwrap_or(Entity::PLACEHOLDER));

            // Set the first 2 frames alpht to 1 and the rest to 0.
            frames.iter().enumerate().for_each(|(i, entity)| {
                if let Some(mut geometry) = world.get_mut::<GeometryTiled>(*entity) {
                    geometry.alpha = if i <= 1 { 1.0 } else { 0.0 };
                }
            });

            frames
        };

        world.insert_resource(AnimationState {
            frames,
            current_frame: 0,
            fading_out: false,
        });

        queue.apply(&mut world);

        let mut update_schedule = Schedule::default();
        update_schedule.add_systems(
            (
                // If any UiAction messages were emitted on the previous frame,
                // handle them first.
                handle_ui_actions,
                update_global_rects,
                emit_widget_messages,
                update_buttons,
                animate_button_shadow,
                rotate_background_alphas,
                update_window_render_items,
            )
                .chain(),
        );

        Ok(Self {
            world,
            update_schedule,
            window_renderer,
        })
    }

    fn spawn_buttons(
        commands: &mut Commands,
        window_entity: Entity,
        sprites: &Sprites,
        window_base: &WindowBase,
    ) {
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

        let spawn_sprite = |commands: &mut Commands,
                            position: glam::IVec2,
                            sprite_name: &str,
                            frame: usize,
                            alpha: f32| {
            let sprite = sprites.get_handle_by_name(sprite_name)?;
            let frame_size = sprite_frame_size(sprite_name, frame)?;

            let entity = commands
                .spawn((
                    SpriteRender {
                        position: position.as_vec2(),
                        alpha,
                        sprite,
                        frame,
                    },
                    ChildOf(window_entity),
                ))
                .id();

            Some((entity, frame_size))
        };

        for button in BUTTONS {
            let position = window_base
                .button_advices
                .get(button.name)
                .map(|button| glam::IVec2::new(button.x, button.y))
                .unwrap_or(glam::IVec2::ZERO);

            let Some((shadow_entity, _)) = spawn_sprite(
                commands,
                position + shadow_offset,
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
                commands,
                position + button_offset,
                button.text_sprite,
                button.text_frame,
                BASE_TEXT_ALPHA,
            ) else {
                continue;
            };
            let Some((pressed_entity, _)) = spawn_sprite(
                commands,
                position + button_offset,
                button.pressed_sprite,
                button.pressed_frame,
                0.0,
            ) else {
                continue;
            };

            commands.spawn((
                Rect {
                    position,
                    size: text_size + UVec2::new(bullet_size.x, 0),
                },
                Widget,
                Button::new(if button.name == "b_exit" {
                    UiAction::ShowHelpWindow(Cow::Borrowed("conf_exit_game"))
                } else {
                    UiAction::Exit
                }),
                MainMenuButtonAnimation {
                    button_offset,
                    shadow_offset,
                    shadow_entity,
                    text_entity,
                    pressed_entity,
                    hover_progress_ms: 0.0,
                },
            ));
        }
    }

    fn spawn_version_label(commands: &mut Commands, window_renderer: &WindowRenderer) {
        // The original engine positions "v1.31" at (635 - text_width, 475 - text_height)
        // in 640x480 mode, placing it in the bottom-right corner. The text is rendered
        // with font_12_point and a golden amber color override (0xffd3a333).
        let version_text = "v1.31";
        let font = Font::TwelvePoint;
        let text_width = window_renderer.measure_text_width(version_text, font);
        let text_height = window_renderer.measure_text_height(version_text, font);
        let position = glam::Vec2::new(635.0 - text_width, 475.0 - text_height);

        commands.spawn(TextRender {
            position,
            text: version_text.to_owned(),
            font,
            color: Some(glam::Vec4::new(
                0xd3 as f32 / 255.0,
                0xa3 as f32 / 255.0,
                0x33 as f32 / 255.0,
                1.0,
            )),
        });
    }
}

impl Scene for MainMenuScene {
    fn resize(&mut self, size: glam::UVec2) {
        self.window_renderer.resize(size);
    }

    fn input_event(&mut self, event: &InputEvent) {
        let message = match *event {
            InputEvent::MouseMove(position) => WindowMessage::MouseMove(position),
            InputEvent::MouseLeave => WindowMessage::MouseLeave,
            InputEvent::MouseDown(button) => WindowMessage::MouseDown(button),
            InputEvent::MouseUp(button) => WindowMessage::MouseUp(button),
            _ => return,
        };
        self.world.write_message(message);
    }

    fn update(&mut self, delta_time: f32) {
        self.world.resource_mut::<DeltaTime>().0 = delta_time;
        self.update_schedule.run(&mut self.world);
    }

    fn render(&mut self, _context: &RenderContext, frame: &mut Frame) {
        let windows = self.world.resource::<WindowManager>().windows.clone();
        let mut render_items =
            std::mem::take(&mut self.world.resource_mut::<RenderSnapshot>().render_items);
        render_items.clear();

        for window in windows {
            if let Some(window_render_items) = self.world.get::<WindowRenderItems>(window) {
                render_items.extend_from(window_render_items);
            }
        }

        self.window_renderer
            .submit_render_items(frame, &render_items);
        self.world.resource_mut::<RenderSnapshot>().render_items = render_items;
    }
}

fn rotate_background_alphas(
    mut state: ResMut<AnimationState>,
    mut geometries: Query<&mut GeometryTiled>,
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

#[inline]
fn hit_test(rect: &GlobalRect, point: IVec2) -> bool {
    point.x >= rect.min.x && point.y >= rect.min.y && point.x < rect.max.x && point.y < rect.max.y
}

fn emit_widget_messages(
    mut window_messages: MessageReader<WindowMessage>,
    widgets: Query<(Entity, &GlobalRect), With<Widget>>,
    mut widget_messages: MessageWriter<WidgetMessage>,
    mut last_mouse_position: Local<Option<glam::UVec2>>,
) {
    let mut mouse_downs = Vec::new();
    let mut mouse_ups = Vec::new();
    let mut has_input_update = false;

    for message in window_messages.read() {
        has_input_update = true;

        match message {
            WindowMessage::MouseMove(position) => {
                let previous = last_mouse_position.map(|p| p.as_ivec2());
                *last_mouse_position = Some(*position);

                for (entity, rect) in widgets.iter() {
                    let was_over = previous.is_some_and(|pos| hit_test(rect, pos));
                    let now_over = hit_test(rect, position.as_ivec2());

                    if now_over && !was_over {
                        widget_messages.write(WidgetMessage::Enter(entity));
                    } else if !now_over && was_over {
                        widget_messages.write(WidgetMessage::Exit(entity));
                    }
                }
            }
            WindowMessage::MouseLeave => {
                if let Some(previous) = last_mouse_position.take() {
                    for (entity, rect) in widgets.iter() {
                        if hit_test(rect, previous.as_ivec2()) {
                            widget_messages.write(WidgetMessage::Exit(entity));
                        }
                    }
                }
            }
            WindowMessage::MouseDown(button) => mouse_downs.push(*button),
            WindowMessage::MouseUp(button) => mouse_ups.push(*button),
        }
    }

    if !has_input_update {
        return;
    }

    if let Some(position) = *last_mouse_position {
        for (entity, rect) in widgets.iter() {
            if hit_test(rect, position.as_ivec2()) {
                for &mouse_button in &mouse_downs {
                    widget_messages.write(WidgetMessage::MouseDown(entity, mouse_button));
                }
                for &mouse_button in &mouse_ups {
                    widget_messages.write(WidgetMessage::MouseUp(entity, mouse_button));
                }
            }
        }
    }
}

fn animate_button_shadow(
    time: Res<DeltaTime>,
    mut buttons: Query<(&GlobalRect, &Button, &mut MainMenuButtonAnimation), With<Widget>>,
    mut renders: Query<&mut SpriteRender>,
) {
    const HOVER_PROGRESS_MAX_MS: f32 = 250.0;
    const HOVER_EXIT_RATE: f32 = 1.0 / 3.0;
    const SHADOW_SLIDE_SCALE: f32 = 0.004;
    const BASE_ALPHA: f32 = 200.0;
    const TEXT_ALPHA_SCALE: f32 = 0.22;
    const SHADOW_ALPHA_SCALE: f32 = 0.8;

    let delta_ms = time.0.max(0.0) * 1000.0;

    for (rect, button, mut animation) in buttons.iter_mut() {
        if button.hovered {
            animation.hover_progress_ms =
                (animation.hover_progress_ms + delta_ms).min(HOVER_PROGRESS_MAX_MS);
        } else {
            animation.hover_progress_ms =
                (animation.hover_progress_ms - delta_ms * HOVER_EXIT_RATE).max(0.0);
        }

        let slide_delta =
            (HOVER_PROGRESS_MAX_MS - animation.hover_progress_ms) * SHADOW_SLIDE_SCALE;
        let text_position = rect.min + animation.button_offset;
        let shadow_delta =
            (animation.shadow_offset - animation.button_offset).as_vec2() * slide_delta;
        let text_alpha =
            (BASE_ALPHA + animation.hover_progress_ms * TEXT_ALPHA_SCALE).round() / 255.0;
        let shadow_alpha =
            (BASE_ALPHA - animation.hover_progress_ms * SHADOW_ALPHA_SCALE).round() / 255.0;

        if let Ok(mut shadow_render) = renders.get_mut(animation.shadow_entity) {
            shadow_render.position = text_position.as_vec2() + shadow_delta;
            shadow_render.alpha = shadow_alpha;
        }

        if let Ok(mut text_render) = renders.get_mut(animation.text_entity) {
            text_render.alpha = if button.pressed { 0.0 } else { text_alpha };
        }

        if let Ok(mut pressed_render) = renders.get_mut(animation.pressed_entity) {
            pressed_render.position = text_position.as_vec2();
            pressed_render.alpha = if button.pressed { text_alpha } else { 0.0 };
        }
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
