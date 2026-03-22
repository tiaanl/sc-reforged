use crate::game::config::parser::{ConfigLine, ConfigLines};

#[derive(Debug, Default)]
pub struct Image {
    pub name: String,
    pub filename: String,
    pub vid_mem: i32,
}

impl From<ConfigLine> for Image {
    fn from(value: ConfigLine) -> Self {
        Self {
            name: value.param(0),
            filename: value.param(1),
            vid_mem: value.param(2),
        }
    }
}

#[derive(Debug, Default)]
pub struct ColorKey {
    rl: u8,
    gl: u8,
    bl: u8,
    rh: u8,
    gh: u8,
    bh: u8,
}

#[derive(Debug, Default)]
pub struct SpriteFrame {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    x_run: i32,
    dx: i32,
}

#[derive(Debug, Default)]
pub struct Sprite3d {
    name: String,
    texture_name: String,
    width: i32,
    height: i32,
    alpha: Option<f32>,
    color_key_enabled: Option<bool>,
    color_key: Option<ColorKey>,
    frames: Vec<SpriteFrame>,
}

impl From<ConfigLine> for Sprite3d {
    fn from(value: ConfigLine) -> Self {
        // SPRITE3D <NAME> <TEXTURENAME> <TXTR_WIDTH> <TXTR_HEIGHT> [<ALPHA>] [<Color Key Enable>] [ <Rl> <Gl> <Bl> <Rh> <Gh> Bh> ]
        //     SPRITEFRAME <x1> <y1> <x2> <y2>
        // ENDDEF

        let name = value.param(0);
        let texture_name = value.param(1);
        let width = value.param(2);
        let height = value.param(3);

        let alpha = value.maybe_param(4);

        let color_key_enabled = value.maybe_param::<i32>(5).map(|i| i != 0);
        let color_key = color_key_enabled.and_then(|e| {
            e.then(|| ColorKey {
                rl: value.param::<i32>(6) as u8,
                gl: value.param::<i32>(7) as u8,
                bl: value.param::<i32>(8) as u8,
                rh: value.param::<i32>(9) as u8,
                gh: value.param::<i32>(10) as u8,
                bh: value.param::<i32>(11) as u8,
            })
        });

        Self {
            name,
            texture_name,
            width,
            height,
            alpha,
            color_key_enabled,
            color_key,
            frames: vec![],
        }
    }
}

#[derive(Debug, Default)]
pub struct AnimSprite {
    name: String,
    image_name: String,
    color_key: Option<ColorKey>,
    frames_descriptor: FrameDescritor,
    frames: Vec<SpriteFrame>,
}

impl From<ConfigLine> for AnimSprite {
    fn from(value: ConfigLine) -> Self {
        // ANIMSPRITE <NAME> <IMAGENAME> [ <-1> | <Rl> <Gl> <Bl> <Rh> <Gh> Bh> ]
        let name = value.param(0);
        let image_name = value.param(1);

        let first = value.param::<i32>(2);

        let color_key = if first == -1 {
            None
        } else {
            Some(ColorKey {
                rl: value.param::<i32>(6) as u8,
                gl: value.param::<i32>(7) as u8,
                bl: value.param::<i32>(8) as u8,
                rh: value.param::<i32>(9) as u8,
                gh: value.param::<i32>(10) as u8,
                bh: value.param::<i32>(11) as u8,
            })
        };

        Self {
            name,
            image_name,
            color_key,
            frames_descriptor: FrameDescritor::default(),
            frames: Vec::default(),
        }
    }
}

#[derive(Debug, Default)]
pub struct AnimSprite3d {
    name: String,
    texture_name: String,
    width: i32,
    height: i32,
    alpha: Option<f32>,
    color_key_enabled: Option<bool>,
    color_key: Option<ColorKey>,

    frame_descriptor: FrameDescritor,
    frame_order: Vec<i32>,

    frames: Vec<SpriteFrame>,
}

impl From<ConfigLine> for AnimSprite3d {
    fn from(value: ConfigLine) -> Self {
        // ANIMSPRITE3D <NAME> <TEXTURENAME> <TXTR_WIDTH> <TXTR_HEIGHT> [<ALPHA>] [<Color Key Enable>] [ Rl> <Gl> <Bl> <Rh> <Gh> Bh> ;]
        let name = value.param(0);
        let texture_name = value.param(1);
        let width = value.param(2);
        let height = value.param(3);

        let alpha = value.maybe_param(4);

        let color_key_enabled = value.maybe_param::<i32>(5).map(|i| i != 0);
        let color_key = color_key_enabled.and_then(|e| {
            e.then(|| ColorKey {
                rl: value.param::<i32>(6) as u8,
                gl: value.param::<i32>(7) as u8,
                bl: value.param::<i32>(8) as u8,
                rh: value.param::<i32>(9) as u8,
                gh: value.param::<i32>(10) as u8,
                bh: value.param::<i32>(11) as u8,
            })
        });

        Self {
            name,
            texture_name,
            width,
            height,
            alpha,
            color_key_enabled,
            color_key,
            frame_descriptor: FrameDescritor::default(),
            frame_order: Vec::default(),
            frames: Vec::default(),
        }
    }
}

#[derive(Debug, Default)]
struct FrameDescritor {
    num_images: i32,
    num_frames: i32,
    frame_rate: i32,
}

impl From<ConfigLine> for FrameDescritor {
    fn from(value: ConfigLine) -> Self {
        // FRAMEDESCRIPTOR <NUM IMAGES> <NUM FRAMES> <FRAME RATE>
        Self {
            num_images: value.param(0),
            num_frames: value.param(1),
            frame_rate: value.param(2),
        }
    }
}

#[derive(Debug, Default)]
pub struct ImageDefs {
    pub images: Vec<Image>,
    pub sprite_3d: Vec<Sprite3d>,
    pub anim_sprite: Vec<AnimSprite>,
    pub anim_sprite_3d: Vec<AnimSprite3d>,
}

impl From<ConfigLines> for ImageDefs {
    fn from(value: ConfigLines) -> Self {
        let mut image_defs = ImageDefs::default();

        #[derive(Debug, Default)]
        enum State {
            #[default]
            None,
            Sprite3d(Sprite3d),
            AnimSprite(AnimSprite),
            AnimSprite3d(AnimSprite3d),
        }

        impl State {
            fn is_none(&self) -> bool {
                matches!(self, Self::None)
            }

            fn open(&mut self, state: State) {
                if !self.is_none() {
                    tracing::warn!("Discarding state for: {:?}", state);
                }

                *self = state;
            }
        }

        let mut state = State::default();

        for line in value.into_iter() {
            match line.key.as_str() {
                "IMAGE" => {
                    state.open(State::None);
                    image_defs.images.push(line.into());
                }

                "SPRITE3D" => state.open(State::Sprite3d(line.into())),
                "ANIMSPRITE3D" => state.open(State::AnimSprite3d(line.into())),
                "ANIMSPRITE" => state.open(State::AnimSprite(line.into())),

                s @ "SPRITEFRAME" | s @ "SPRITEFRAME_XRUN" | s @ "SPRITEFRAME_DXRUN" => {
                    let frames = match state {
                        State::Sprite3d(Sprite3d { ref mut frames, .. })
                        | State::AnimSprite(AnimSprite { ref mut frames, .. })
                        | State::AnimSprite3d(AnimSprite3d { ref mut frames, .. }) => frames,
                        _ => {
                            tracing::warn!("SPRITEFRAME* without a SPRITE*");
                            continue;
                        }
                    };

                    let sprite_frame = match s {
                        "SPRITEFRAME" => SpriteFrame {
                            x1: line.param(0),
                            y1: line.param(1),
                            x2: line.param(2),
                            y2: line.param(3),
                            x_run: 0,
                            dx: 0,
                        },
                        "SPRITEFRAME_XRUN" => SpriteFrame {
                            x1: line.param(0),
                            y1: line.param(1),
                            x2: line.param(2),
                            y2: line.param(3),
                            x_run: line.param(4),
                            dx: 0,
                        },
                        "SPRITEFRAME_DXRUN" => SpriteFrame {
                            x1: line.param(0),
                            y1: line.param(1),
                            x2: line.param(2),
                            y2: line.param(3),
                            x_run: line.param(4),
                            dx: line.param(5),
                        },
                        _ => unreachable!("already checked"),
                    };

                    frames.push(sprite_frame);
                }

                "FRAMEDESCRIPTOR" => match state {
                    State::AnimSprite(ref mut anim_sprite) => {
                        anim_sprite.frames_descriptor = line.into();
                    }
                    State::AnimSprite3d(ref mut anim_sprite_3d) => {
                        anim_sprite_3d.frame_descriptor = line.into();
                    }
                    _ => {
                        tracing::warn!("Found FRAMEDESCRIPTOR, but not in correct state! {state:?}")
                    }
                },

                "FRAMEORDER" => {
                    use crate::game::config::parser::ConfigToken;

                    let frame_order = line
                        .params()
                        .iter()
                        .map(|s| match s {
                            ConfigToken::Number(number) => *number,
                            _ => {
                                tracing::warn!("FRAMEORDER has invalid values: {:?}", line);
                                0
                            }
                        })
                        .collect();

                    match state {
                        State::AnimSprite3d(ref mut anim_sprite_3d) => {
                            anim_sprite_3d.frame_order = frame_order;
                        }
                        _ => tracing::warn!(
                            "Found FRAMEDESCRIPTOR, but not in correct state! {state:?}"
                        ),
                    }
                }

                "ENDDEF" => {
                    let state = std::mem::take(&mut state);
                    match state {
                        State::Sprite3d(sprite_3d) => image_defs.sprite_3d.push(sprite_3d),
                        State::AnimSprite(anim_sprite) => image_defs.anim_sprite.push(anim_sprite),
                        State::AnimSprite3d(anim_sprite_3d) => {
                            image_defs.anim_sprite_3d.push(anim_sprite_3d)
                        }
                        _ => {
                            tracing::warn!("Found ENDDEF, but not in correct state! {state:?}");
                        }
                    }
                }

                _ => tracing::warn!("Unexpected config value: {line:?}, state: {state:?}"),
            }
        }

        image_defs
    }
}
