use ahash::HashMap;
use glam::IVec2;
use smallvec::SmallVec;

use crate::game::{
    config::parser::{ConfigLine, ConfigLines, ConfigToken},
    ui::{Rect, windows::window_manager::WindowLayoutContext},
};

#[derive(Debug)]
pub enum Atom {
    Number(i32),
    UserVar(String),
    SystemVar(String),
}

#[derive(Debug)]
pub enum Adjustment {
    Add(i32),
    Subtract(i32),
    Multiply(i32),
    Divide(i32),
}

#[derive(Debug)]
pub struct Expr {
    atom: Atom,
    adjustments: SmallVec<[Adjustment; 0]>,
}

impl Expr {
    pub const ZERO: Expr = Expr {
        atom: Atom::Number(0),
        adjustments: SmallVec::new_const(),
    };
}

impl Default for Expr {
    #[inline]
    fn default() -> Self {
        Self::ZERO
    }
}

impl From<ConfigToken> for Expr {
    fn from(value: ConfigToken) -> Self {
        let mut result = Self::ZERO;

        match value {
            ConfigToken::String(value) => {
                let (prefix, value) = value.split_at(1);
                if prefix == "$" {
                    result.atom = Atom::UserVar(value.into());
                } else if prefix == "%" {
                    result.atom = Atom::SystemVar(value.into());
                } else {
                    panic!("Variable without source specifier!");
                }
            }
            ConfigToken::Float(value) => {
                tracing::warn!(
                    "Floating point values for supported in expressions: {}",
                    value
                );
            }
            ConfigToken::Number(value) => result.atom = Atom::Number(value),
        }

        result
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
    pub x: Expr,
    pub y: Expr,
    pub dx: Expr,
    pub dy: Expr,
    pub sprite: Option<ButtonAdviceSprite>,
}

#[derive(Debug)]
pub struct Vertex {
    pub x: i32,
    pub y: i32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
    pub u: Option<i32>,
    pub v: Option<i32>,
}

#[derive(Debug)]
pub struct Polygon {
    pub i0: u32,
    pub i1: u32,
    pub i2: u32,
}

#[derive(Debug)]
pub struct GeometryNormal {
    pub texture: String,
    pub texture_pack_dx: i32,
    pub texture_pack_dy: i32,
    pub blend_mode: String,
    pub bilinear_filtering: bool,

    pub vertices: Vec<Vertex>,
    pub polygons: Vec<Polygon>,
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
    pub dx: Expr,
    pub dy: Expr,
    pub render_dx: Expr,
    pub render_dy: Expr,
    pub reload_on_mode_switch: bool,

    pub button_advices: HashMap<String, ButtonAdvice>,
    pub geometries: Vec<Geometry>,
    pub ivars: HashMap<String, Expr>,
}

impl WindowBase {
    pub fn resolve(&self, expr: &Expr, context: &WindowLayoutContext) -> Option<i32> {
        match expr.atom {
            Atom::Number(value) => Some(value),
            Atom::UserVar(ref value) => {
                let expr = self.ivars.get(value)?;
                self.resolve(expr, context)
            }
            Atom::SystemVar(ref value) => match value.as_str() {
                "screen_dx" => Some(context.screen_dx),
                "screen_dy" => Some(context.screen_dy),
                _ => None,
            },
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

impl From<ConfigLines> for WindowBase {
    fn from(lines: ConfigLines) -> Self {
        #[derive(Default)]
        enum State {
            #[default]
            None,
            GeometryNormal(GeometryNormal),
            GeometryTiled(GeometryTiled),
        }

        let mut result = WindowBase {
            name: String::new(),
            dx: Expr::ZERO,
            dy: Expr::ZERO,
            render_dx: Expr::ZERO,
            render_dy: Expr::ZERO,
            reload_on_mode_switch: false,
            button_advices: HashMap::default(),
            geometries: Vec::default(),
            ivars: HashMap::default(),
        };

        let mut state = State::default();

        for line in lines.into_iter() {
            match &mut state {
                State::None => match line.key.as_str() {
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
                        let value = line.param(1);

                        result.ivars.insert(key, value);
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

                    "WINDOW_BASE_RELOAD_ON_MODE_SWITCH" => {
                        result.reload_on_mode_switch = line.string(0).eq_ignore_ascii_case("true")
                    }

                    "WINDOW_BASE_GEOMETRY_TILED" => {
                        state = State::GeometryTiled(GeometryTiled::default());
                    }

                    _ => {
                        tracing::warn!("Invalid key for WindowBase: {}", line.key);
                    }
                },

                State::GeometryNormal(geometry_normal) => todo!(),

                State::GeometryTiled(geometry_tiled) => match line.key.as_str() {
                    "WINDOW_BASE_GEOMETRY_TILED" => {
                        let geometry = std::mem::take(geometry_tiled);
                        result.geometries.push(Geometry::Tiled(geometry));
                    }

                    "GEOMETRY_JPG_NAME" => geometry_tiled.jpg_name = line.param(0),

                    "GEOMETRY_JPG_DIMENSIONS" => {
                        geometry_tiled.dimensions = IVec2::new(line.param(0), line.param(1))
                    }

                    "GEOMETRY_CHUNK_DIMENSIONS" => {
                        geometry_tiled.chunk_dimensions = IVec2::new(line.param(0), line.param(1))
                    }

                    _ => {
                        tracing::warn!("Invalid key for GeometryTiled: {}", line.key);
                    }
                },
            }
        }

        match state {
            State::None => {}
            State::GeometryNormal(geometry_normal) => todo!(),
            State::GeometryTiled(geometry_tiled) => {
                result.geometries.push(Geometry::Tiled(geometry_tiled));
            }
        }

        result
    }
}
