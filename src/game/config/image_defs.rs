#![allow(dead_code)]

use crate::game::{asset_loader::AssetError, config::ConfigFile};

#[derive(Debug, Default)]
pub struct Image {
    pub name: String,
    pub filename: String,
    pub vid_mem: u32,
}

impl Image {
    fn from_params(params: &[&str]) -> Self {
        // IMAGE <NAME> <FILENAME> <vidmem 1=true>
        Self {
            name: params[1].to_string(),
            filename: params[2].to_string(),
            vid_mem: params[3].parse().unwrap(),
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

impl ColorKey {
    fn from_params(params: &[&str]) -> Self {
        let rl = params[0].parse().unwrap();
        let gl = params[1].parse().unwrap();
        let bl = params[2].parse().unwrap();
        let rh = params[3].parse().unwrap();
        let gh = params[4].parse().unwrap();
        let bh = params[5].parse().unwrap();

        Self {
            rl,
            gl,
            bl,
            rh,
            gh,
            bh,
        }
    }
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

impl SpriteFrame {
    fn from_params(params: &[&str]) -> Self {
        Self {
            x1: params[0].parse().unwrap(),
            y1: params[1].parse().unwrap(),
            x2: params[2].parse().unwrap(),
            y2: params[3].parse().unwrap(),
            x_run: 0,
            dx: 0,
        }
    }

    fn from_params_x_run(params: &[&str]) -> Self {
        Self {
            x_run: params[4].parse().unwrap(),
            ..Self::from_params(params)
        }
    }

    fn from_params_dx_run(params: &[&str]) -> Self {
        Self {
            dx: params[5].parse().unwrap(),
            ..Self::from_params_x_run(params)
        }
    }
}

#[derive(Debug, Default)]
pub struct Sprite3d {
    name: String,
    texture_name: String,
    width: u32,
    height: u32,
    alpha: Option<f32>,
    color_key_enabled: Option<bool>,
    color_key: Option<ColorKey>,
    frames: Vec<SpriteFrame>,
}

impl Sprite3d {
    fn from_params(params: &[&str]) -> Self {
        // SPRITE3D <NAME> <TEXTURENAME> <TXTR_WIDTH> <TXTR_HEIGHT> [<ALPHA>] [<Color Key Enable>] [ <Rl> <Gl> <Bl> <Rh> <Gh> Bh> ]
        //     SPRITEFRAME <x1> <y1> <x2> <y2>
        // ENDDEF

        let name = params[1].to_string();
        let texture_name = params[2].to_string();
        let width = params[3].parse().unwrap();
        let height = params[4].parse().unwrap();

        let alpha = if params.len() > 5 {
            Some(params[5].parse().unwrap())
        } else {
            None
        };

        let color_key_enabled = if params.len() > 6 {
            Some(params[6].parse::<i32>().unwrap() != 0)
        } else {
            None
        };

        let color_key = if params.len() > 7 {
            Some(ColorKey {
                rl: params[7].parse().unwrap(),
                gl: params[8].parse().unwrap(),
                bl: params[9].parse().unwrap(),
                rh: params[10].parse().unwrap(),
                gh: params[11].parse().unwrap(),
                bh: params[12].parse().unwrap(),
            })
        } else {
            None
        };

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
struct AnimSprite {
    name: String,
    image_name: String,
    color_key: Option<ColorKey>,
    frames_descriptor: FrameDescritor,
    frames: Vec<SpriteFrame>,
}

impl AnimSprite {
    fn from_params(params: &[&str]) -> Self {
        // ANIMSPRITE <NAME> <IMAGENAME> [ <-1> | <Rl> <Gl> <Bl> <Rh> <Gh> Bh> ]
        let name = params[0].to_string();
        let image_name = params[1].to_string();

        let color_key = if params[2].parse::<i32>().unwrap() == -1 {
            None
        } else {
            Some(ColorKey::from_params(&params[2..]))
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
struct AnimSprite3d {
    name: String,
    texture_name: String,
    width: u32,
    height: u32,
    alpha: Option<f32>,
    color_key_enabled: bool,
    color_key: Option<ColorKey>,

    frame_descriptor: FrameDescritor,
    frame_order: Vec<u32>,

    frames: Vec<SpriteFrame>,
}

impl AnimSprite3d {
    fn from_params(params: &[&str]) -> Self {
        // ANIMSPRITE3D <NAME> <TEXTURENAME> <TXTR_WIDTH> <TXTR_HEIGHT> [<ALPHA>] [<Color Key Enable>] [ Rl> <Gl> <Bl> <Rh> <Gh> Bh> ;]
        let name = params[0].to_string();
        let texture_name = params[1].to_string();
        let width = params[2].parse().unwrap();
        let height = params[3].parse().unwrap();

        let alpha = if params.len() > 4 {
            Some(params[4].parse().unwrap())
        } else {
            None
        };

        let color_key_enabled = if params.len() > 5 {
            params[5].parse::<i32>().unwrap() == 1
        } else {
            false
        };

        let color_key = if params.len() > 6 {
            Some(ColorKey::from_params(&params[6..]))
        } else {
            None
        };

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
    num_images: u32,
    num_frames: u32,
    frame_rate: u32,
}

impl FrameDescritor {
    fn from_params(params: &[&str]) -> Self {
        // FRAMEDESCRIPTOR <NUM IMAGES> <NUM FRAMES> <FRAME RATE>
        Self {
            num_images: params[0].parse().unwrap(),
            num_frames: params[1].parse().unwrap(),
            frame_rate: params[2].parse().unwrap(),
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

pub fn read_image_defs(data: &str) -> ImageDefs {
    let mut image_defs = ImageDefs::default();

    let mut config = ConfigFile::new(data);

    #[derive(Debug)]
    enum State {
        None,
        Sprite3d(Sprite3d),
        AnimSprite(AnimSprite),
        AnimSprite3d(AnimSprite3d),
    }
    let mut state = State::None;

    while let Some(current) = config.current() {
        match current[0] {
            s if s.starts_with(';') => {}

            "IMAGE" => {
                image_defs.images.push(Image::from_params(current));
            }

            "SPRITE3D" => {
                let sprite_3d = Sprite3d::from_params(current);
                state = State::Sprite3d(sprite_3d);
            }

            s @ "SPRITEFRAME" | s @ "SPRITEFRAME_XRUN" | s @ "SPRITEFRAME_DXRUN" => {
                let sprite_frame = match s {
                    "SPRITEFRAME" => SpriteFrame::from_params(&current[1..]),
                    "SPRITEFRAME_XRUN" => SpriteFrame::from_params_x_run(&current[1..]),
                    "SPRITEFRAME_DXRUN" => SpriteFrame::from_params_dx_run(&current[1..]),
                    _ => unreachable!("already checked"),
                };
                match state {
                    State::Sprite3d(Sprite3d { ref mut frames, .. })
                    | State::AnimSprite(AnimSprite { ref mut frames, .. })
                    | State::AnimSprite3d(AnimSprite3d { ref mut frames, .. }) => {
                        frames.push(sprite_frame);
                    }
                    _ => panic!("Found SPRITEFRAME, but not in correct state! {:?}", state),
                }
            }

            "ENDDEF" => {
                let state = std::mem::replace(&mut state, State::None);
                match state {
                    State::Sprite3d(sprite_3d) => image_defs.sprite_3d.push(sprite_3d),
                    State::AnimSprite(anim_sprite) => image_defs.anim_sprite.push(anim_sprite),
                    State::AnimSprite3d(anim_sprite_3d) => {
                        image_defs.anim_sprite_3d.push(anim_sprite_3d)
                    }
                    _ => panic!("Found ENDDEF, but not in correct state! {:?}", state),
                }
            }

            "ANIMSPRITE3D" => {
                let anim_sprite_3d = AnimSprite3d::from_params(&current[1..]);
                state = State::AnimSprite3d(anim_sprite_3d);
            }

            "FRAMEDESCRIPTOR" => {
                let frame_descriptor = FrameDescritor::from_params(&current[1..]);
                match state {
                    State::AnimSprite(ref mut anim_sprite) => {
                        //
                        anim_sprite.frames_descriptor = frame_descriptor;
                    }
                    State::AnimSprite3d(ref mut anim_sprite_3d) => {
                        anim_sprite_3d.frame_descriptor = frame_descriptor;
                    }
                    _ => panic!(
                        "Found FRAMEDESCRIPTOR, but not in correct state! {:?}",
                        state
                    ),
                }
            }

            "FRAMEORDER" => {
                let frame_order = current[1..].iter().map(|s| s.parse().unwrap()).collect();
                match state {
                    State::AnimSprite3d(ref mut anim_sprite_3d) => {
                        anim_sprite_3d.frame_order = frame_order;
                    }
                    _ => panic!(
                        "Found FRAMEDESCRIPTOR, but not in correct state! {:?}",
                        state
                    ),
                }
            }

            "ANIMSPRITE" => {
                let anim_sprite = AnimSprite::from_params(&current[1..]);
                state = State::AnimSprite(anim_sprite);
            }

            _ => panic!(
                "Unexpected config value: {:?}, state: {:?}",
                current.join(", "),
                state
            ),
        }
        config.next();
    }

    image_defs
}

impl TryFrom<String> for ImageDefs {
    type Error = AssetError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(read_image_defs(&value))
    }
}
