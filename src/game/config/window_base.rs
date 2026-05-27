use ahash::HashMap;
use glam::IVec2;
use smallvec::SmallVec;

use crate::game::{
    config::parser::{ConfigLine, ConfigLines, ConfigToken},
    ui::{
        Rect, render::window_renderer::TiledGeometry, windows::window_manager::WindowLayoutContext,
    },
};

#[derive(Debug)]
pub enum Atom {
    Number(i32),
    UserVar(String),
    SystemVar {
        name: String,
        adjustments: SmallVec<[Adjustment; 0]>,
    },
}

impl Atom {
    pub const ZERO: Self = Self::Number(0);
}

impl Default for Atom {
    #[inline]
    fn default() -> Self {
        Self::ZERO
    }
}

impl From<ConfigToken> for Atom {
    fn from(value: ConfigToken) -> Self {
        match value {
            ConfigToken::String(value) => parse_atom_string(&value).unwrap_or(Self::ZERO),
            ConfigToken::Float(value) => {
                tracing::warn!(
                    "Floating point values for supported in expressions: {}",
                    value
                );
                Self::ZERO
            }
            ConfigToken::Number(value) => Self::Number(value),
        }
    }
}

#[derive(Debug)]
pub enum Adjustment {
    Add(i32),
    Subtract(i32),
    Multiply(i32),
    Divide(i32),
}

impl Adjustment {
    pub fn apply(&self, value: &mut i32) {
        match self {
            Adjustment::Add(i) => *value += i,
            Adjustment::Subtract(i) => *value -= i,
            Adjustment::Multiply(i) => *value *= i,
            Adjustment::Divide(i) => *value /= i,
        }
    }
}

#[derive(Debug)]
pub struct IVar {
    atom: Atom,
    adjustments: SmallVec<[Adjustment; 0]>,
}

impl IVar {
    pub const ZERO: IVar = IVar {
        atom: Atom::Number(0),
        adjustments: SmallVec::new_const(),
    };
}

impl Default for IVar {
    #[inline]
    fn default() -> Self {
        Self::ZERO
    }
}

#[derive(Debug)]
pub struct ButtonAdviceSprite {
    pub as_3d_index: i32,
    pub unpressed: i32,
    pub pressed: i32,
    pub middle: i32,
}

#[derive(Debug)]
pub struct ButtonAdvice {
    pub id: String,
    pub x: Atom,
    pub y: Atom,
    pub dx: Atom,
    pub dy: Atom,
    pub sprite: Option<ButtonAdviceSprite>,
}

#[derive(Debug, Default)]
pub struct GeometryVertex {
    pub x: Atom,
    pub y: Atom,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
    pub u: Option<i32>,
    pub v: Option<i32>,
}

#[derive(Debug)]
pub struct GeometryPolygon {
    pub i0: i32,
    pub i1: i32,
    pub i2: i32,
}

#[derive(Debug, Default)]
pub struct GeometryNormal {
    pub texture: String,
    pub texture_pack_dx: i32,
    pub texture_pack_dy: i32,
    pub blend_mode: String,
    pub bilinear_filtering: bool,

    pub vertices: Vec<GeometryVertex>,
    pub polygons: Vec<GeometryPolygon>,
}

#[derive(Debug, Default)]
pub struct GeometryTiled {
    pub jpg_name: String,
    pub dimensions: IVec2,
    pub chunk_dimensions: IVec2,
    pub bilinear_filtering: bool,
}

#[derive(Debug)]
pub enum Geometry {
    Normal(GeometryNormal),
    Tiled(GeometryTiled),
}

#[derive(Debug)]
pub struct WindowBase {
    pub name: String,
    pub dx: Atom,
    pub dy: Atom,
    pub render_dx: Atom,
    pub render_dy: Atom,
    pub reload_on_mode_switch: bool,

    pub button_advices: HashMap<String, ButtonAdvice>,
    pub geometries: Vec<Geometry>,
    pub ivars: HashMap<String, IVar>,
}

impl WindowBase {
    pub fn resolve(&self, atom: &Atom, context: &WindowLayoutContext) -> Option<i32> {
        match *atom {
            Atom::Number(value) => Some(value),
            Atom::UserVar(ref value) => {
                let ivar = self.ivars.get(value)?;
                // Resolve the ivar atom.
                let mut value = self.resolve(&ivar.atom, context)?;
                for adjustment in ivar.adjustments.iter() {
                    adjustment.apply(&mut value);
                }
                Some(value)
            }
            Atom::SystemVar {
                ref name,
                ref adjustments,
            } => {
                let mut value = match name.as_str() {
                    "screen_dx" => context.screen_dx,
                    "screen_dy" => context.screen_dy,
                    _ => return None,
                };
                for adjustment in adjustments.iter() {
                    adjustment.apply(&mut value);
                }
                Some(value)
            }
        }
    }

    pub fn resolve_layout_rect(&self, context: &WindowLayoutContext) -> Rect {
        let x = self.resolve(&self.render_dx, context).unwrap_or(0);
        let y = self.resolve(&self.render_dy, context).unwrap_or(0);

        let width = self.resolve(&self.dx, context).unwrap_or(640);
        let height = self.resolve(&self.dy, context).unwrap_or(480);

        Rect {
            position: IVec2::new(x, y),
            size: IVec2::new(width, height),
        }
    }
}

fn parse_atom_string(value: &str) -> Option<Atom> {
    let mut parts = value.split(',');
    let head = parts.next()?.trim();

    if let Some(value) = head.strip_prefix('$') {
        if parts.next().is_some() {
            tracing::warn!("Inline adjustments are only supported for system vars: {head}");
        }
        return Some(Atom::UserVar(value.into()));
    }

    if let Some(value) = head.strip_prefix('%') {
        let mut adjustments = SmallVec::<[Adjustment; 0]>::new();

        for part in parts {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            let (op, rhs) = part.split_at(1);
            let rhs = rhs.trim().parse::<i32>().ok()?;
            let adjustment = match op {
                "+" => Adjustment::Add(rhs),
                "-" => Adjustment::Subtract(rhs),
                "*" => Adjustment::Multiply(rhs),
                "/" => Adjustment::Divide(rhs),
                _ => return None,
            };
            adjustments.push(adjustment);
        }

        return Some(Atom::SystemVar {
            name: value.into(),
            adjustments,
        });
    }

    head.parse::<i32>().ok().map(Atom::Number)
}

impl From<ConfigLines> for WindowBase {
    fn from(lines: ConfigLines) -> Self {
        #[derive(Default)]
        enum State {
            #[default]
            None,
            GeometryNormal(GeometryNormal),
            GeometryTiled(GeometryTiled),
        }

        impl State {
            fn set(&mut self, result: &mut WindowBase, state: State) {
                match self {
                    State::None => {}
                    State::GeometryNormal(geometry) => result
                        .geometries
                        .push(Geometry::Normal(std::mem::take(geometry))),
                    State::GeometryTiled(geometry) => result
                        .geometries
                        .push(Geometry::Tiled(std::mem::take(geometry))),
                }

                *self = state;
            }

            fn with_tiled_geometry(&mut self, mut f: impl FnMut(&mut GeometryTiled)) {
                if let Self::GeometryTiled(geometry) = self {
                    f(geometry);
                }
            }

            fn with_normal_geometry(&mut self, mut f: impl FnMut(&mut GeometryNormal)) {
                if let Self::GeometryNormal(geometry) = self {
                    f(geometry);
                } else {
                    tracing::warn!("Expected normal geometry!");
                }
            }
        }

        let mut result = WindowBase {
            name: String::new(),
            dx: Atom::ZERO,
            dy: Atom::ZERO,
            render_dx: Atom::ZERO,
            render_dy: Atom::ZERO,
            reload_on_mode_switch: false,
            button_advices: HashMap::default(),
            geometries: Vec::default(),
            ivars: HashMap::default(),
        };

        let mut state = State::default();

        let mut lines = lines.into_iter().peekable();

        loop {
            let Some(line) = lines.next() else {
                break;
            };

            match line.key.as_str() {
                "WINDOW_BASE" => result.name = line.param(0),

                "WINDOW_BASE_DX" => result.dx = line.param(0),
                "WINDOW_BASE_DY" => result.dy = line.param(0),

                "WINDOW_BASE_RENDER_DX" => result.render_dx = line.param(0),
                "WINDOW_BASE_RENDER_DY" => result.render_dy = line.param(0),

                "DEFINE_BUTTON_ADVICE" => {
                    // DEFINE_BUTTON_ADVICE
                    //   <string_id>
                    //   <@i.x>
                    //   <@i.y>
                    //   <@i.dx>
                    //   <@i.dy>
                    //   [
                    //     <@i.as3d index>
                    //     <@i.unpressed/top frame>
                    //     <@i.pressed/bottom frame>
                    //     <@i.middle frame>
                    //   ]

                    let sprite = line.maybe_param(5).map(|index| ButtonAdviceSprite {
                        as_3d_index: index,
                        unpressed: line.param(6),
                        pressed: line.param(7),
                        middle: line.param(8),
                    });

                    let button_advice = ButtonAdvice {
                        id: line.param(0),
                        x: line.param(1),
                        y: line.param(2),
                        dx: line.param(3),
                        dy: line.param(4),
                        sprite,
                    };

                    result
                        .button_advices
                        .insert(button_advice.id.clone(), button_advice);
                }

                "DEFINE_USER_IVAR" => {
                    // DEFINE_USER_IVAR
                    //   <{$}string_id>
                    //   <@i.initial value>

                    let key = line.string(0);
                    let atom = line.param(1);

                    result.ivars.insert(
                        key,
                        IVar {
                            atom,
                            ..Default::default()
                        },
                    );
                }

                "MODIFY_USER_IVAR" => {
                    let key = line.string(0);

                    if let Some(ivar) = result.ivars.get_mut(&key) {
                        let adjustment = match line.string(1).as_str() {
                            "-" => Adjustment::Subtract(line.param(2)),
                            "+" => Adjustment::Add(line.param(2)),
                            "*" => Adjustment::Multiply(line.param(2)),
                            "/" => Adjustment::Divide(line.param(2)),

                            _ => {
                                tracing::warn!(
                                    "Invalid adjustment for ivar \"{}\": {}",
                                    line.string(0),
                                    line.string(1)
                                );
                                continue;
                            }
                        };

                        ivar.adjustments.push(adjustment);
                    }
                }

                "WINDOW_BASE_RELOAD_ON_MODE_SWITCH" => result.reload_on_mode_switch = line.param(0),

                "WINDOW_BASE_GEOMETRY_TILED" => {
                    state.set(&mut result, State::GeometryTiled(GeometryTiled::default()));
                }

                "WINDOW_BASE_GEOMETRY" => {
                    state.set(
                        &mut result,
                        State::GeometryNormal(GeometryNormal::default()),
                    );
                }

                "GEOMETRY_BLEND_MODE" => {
                    state.with_normal_geometry(|geometry| geometry.blend_mode = line.param(0))
                }

                "GEOMETRY_BILINEAR_FILTERING" => match state {
                    State::None => {
                        tracing::warn!("Bilinear filtering specified outside of geometry block!")
                    }
                    State::GeometryNormal(ref mut geometry) => {
                        geometry.bilinear_filtering = line.param(0)
                    }
                    State::GeometryTiled(ref mut geometry) => {
                        geometry.bilinear_filtering = line.param(0)
                    }
                },

                "GEOMETRY_TEXTURE" => {
                    state.with_normal_geometry(|geometry| geometry.texture = line.param(0))
                }

                "GEOMETRY_TEXTURE_PACK_DX" => {
                    state.with_normal_geometry(|geometry| geometry.texture_pack_dx = line.param(0))
                }

                "GEOMETRY_TEXTURE_PACK_DY" => {
                    state.with_normal_geometry(|geometry| geometry.texture_pack_dy = line.param(0))
                }

                "GEOMETRY_VERTICES" => state.with_normal_geometry(|geometry| {
                    let mut count: i32 = line.param(0);

                    loop {
                        if count == 0 {
                            break;
                        }

                        let Some(line) = lines.peek() else {
                            break;
                        };

                        if line.key != "GEOMETRY_VERTEX" {
                            break;
                        }

                        let Some(line) = lines.next() else {
                            break;
                        };

                        count -= 1;

                        geometry.vertices.push(GeometryVertex {
                            x: line.param(0),
                            y: line.param(1),
                            r: line.param(2),
                            g: line.param(3),
                            b: line.param(4),
                            a: line.param(5),
                            u: line.maybe_param(6),
                            v: line.maybe_param(7),
                        });
                    }
                }),

                "GEOMETRY_POLYGONS" => state.with_normal_geometry(|geometry| {
                    let mut count: i32 = line.param(0);
                    loop {
                        if count == 0 {
                            break;
                        }

                        let Some(line) = lines.peek() else {
                            break;
                        };

                        if line.key != "GEOMETRY_POLYGON" {
                            break;
                        }

                        let Some(line) = lines.next() else {
                            break;
                        };

                        count -= 1;

                        geometry.polygons.push(GeometryPolygon {
                            i0: line.param(0),
                            i1: line.param(1),
                            i2: line.param(2),
                        });
                    }
                }),

                "GEOMETRY_JPG_NAME" => {
                    state.with_tiled_geometry(|geometry| geometry.jpg_name = line.param(0));
                }

                "GEOMETRY_JPG_DIMENSIONS" => {
                    state.with_tiled_geometry(|geometry| {
                        geometry.dimensions = IVec2::new(line.param(0), line.param(1))
                    });
                }

                "GEOMETRY_CHUNK_DIMENSIONS" => {
                    state.with_tiled_geometry(|geometry| {
                        geometry.chunk_dimensions = IVec2::new(line.param(0), line.param(1))
                    });
                }

                _ => {
                    tracing::warn!("Invalid key for GeometryTiled: {}", line.key)
                }
            }
        }

        match state {
            State::None => {}
            State::GeometryNormal(geometry_normal) => {
                result.geometries.push(Geometry::Normal(geometry_normal));
            }
            State::GeometryTiled(geometry_tiled) => {
                result.geometries.push(Geometry::Tiled(geometry_tiled));
            }
        }

        result
    }
}
