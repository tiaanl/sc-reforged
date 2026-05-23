use ahash::HashMap;
use glam::IVec2;

use crate::game::{
    config::parser::{ConfigLine, ConfigLines, ConfigToken},
    ui::Rect,
};

/// Sentinel placed in resolved vertex UVs for the `AUTO` keyword.
pub const AUTO_UV: i32 = i32::MIN;

/// Resolution context for [`WindowBase::layout`]: the named values that `%foo`
/// substitutions in window-base expressions look up.
#[derive(Debug, Clone, Copy)]
pub struct WindowLayoutContext {
    pub screen_dx: i32,
    pub screen_dy: i32,
}

impl WindowLayoutContext {
    pub fn from_logical_size(size: IVec2) -> Self {
        Self {
            screen_dx: size.x,
            screen_dy: size.y,
        }
    }

    fn lookup(&self, name: &str) -> Option<i32> {
        match name {
            "screen_dx" => Some(self.screen_dx),
            "screen_dy" => Some(self.screen_dy),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------------

/// The base of an [`IExpr`]: a plain number, a `%name` substitution (looked up
/// in the resolution context), or a `$name` reference into the [`IVars`] table.
#[derive(Debug, Clone)]
pub enum Atom {
    Number(i32),
    Var(String),
    IVar(String),
}

#[derive(Debug, Clone, Copy)]
pub enum AdjOp {
    Add(i32),
    Sub(i32),
    Mul(i32),
    Div(i32),
}

impl AdjOp {
    fn apply(self, lhs: i32) -> i32 {
        match self {
            AdjOp::Add(v) => lhs + v,
            AdjOp::Sub(v) => lhs - v,
            AdjOp::Mul(v) => lhs * v,
            AdjOp::Div(v) if v != 0 => lhs / v,
            AdjOp::Div(_) => lhs,
        }
    }

    fn parse(op: &str, operand: i32) -> Option<Self> {
        Some(match op {
            "+" => AdjOp::Add(operand),
            "-" => AdjOp::Sub(operand),
            "*" => AdjOp::Mul(operand),
            "/" => AdjOp::Div(operand),
            _ => return None,
        })
    }
}

/// An unevaluated window-base expression: an atom plus a chain of integer
/// adjustments (`%screen_dx,-198`, `$foo,/2,-64`, etc.).
#[derive(Debug, Clone)]
pub struct IExpr {
    atom: Atom,
    adjustments: Vec<AdjOp>,
}

impl IExpr {
    pub const fn zero() -> Self {
        Self {
            atom: Atom::Number(0),
            adjustments: Vec::new(),
        }
    }

    pub fn number(value: i32) -> Self {
        Self {
            atom: Atom::Number(value),
            adjustments: Vec::new(),
        }
    }
}

/// Parsed UV component: either the `AUTO` keyword or an expression.
#[derive(Debug, Clone)]
pub enum UvExpr {
    Auto,
    Expr(IExpr),
}

/// The set of `$name` user ivars defined by a window base.
#[derive(Debug, Default)]
pub struct IVars {
    ivars: HashMap<String, IExpr>,
}

impl IVars {
    /// `DEFINE_USER_IVAR $name <expr>`
    pub fn define(&mut self, line: &ConfigLine) {
        let Some(name) = line.maybe_param::<String>(0) else {
            return;
        };
        let Some(expr) = parse_expr_param(line, 1) else {
            tracing::warn!("DEFINE_USER_IVAR {name}: missing value");
            return;
        };
        self.ivars
            .insert(strip_ivar_prefix(&name).to_string(), expr);
    }

    /// `MODIFY_USER_IVAR $name <op> <value-expr>` — appends an adjustment to
    /// the existing ivar's expression.
    pub fn modify(&mut self, line: &ConfigLine) {
        let Some(name) = line.maybe_param::<String>(0) else {
            return;
        };
        let op = line.string(1);
        let Some(operand_expr) = parse_expr_param(line, 2) else {
            tracing::warn!("MODIFY_USER_IVAR {name}: missing value");
            return;
        };

        // For now we only support modify-by-constant — that's all the bases use.
        let operand = match (&operand_expr.atom, operand_expr.adjustments.as_slice()) {
            (Atom::Number(n), []) => *n,
            _ => {
                tracing::warn!("MODIFY_USER_IVAR {name}: non-constant operand unsupported");
                return;
            }
        };

        let key = strip_ivar_prefix(&name);
        let Some(existing) = self.ivars.get_mut(key) else {
            tracing::warn!("MODIFY_USER_IVAR on undefined ivar {name}");
            return;
        };
        let Some(adj) = AdjOp::parse(&op, operand) else {
            tracing::warn!("MODIFY_USER_IVAR {name}: unknown op {op}");
            return;
        };
        existing.adjustments.push(adj);
    }

    /// Resolve an expression against this ivar table and a context struct of
    /// `%name` substitutions (e.g. `screen_dx`).
    pub fn eval(&self, expr: &IExpr, ctx: &WindowLayoutContext) -> Option<i32> {
        let mut acc = match &expr.atom {
            Atom::Number(n) => *n,
            Atom::Var(name) => match ctx.lookup(name) {
                Some(v) => v,
                None => {
                    tracing::warn!("Unknown %-substitution: %{name}");
                    return None;
                }
            },
            Atom::IVar(name) => {
                let referenced = self.ivars.get(name).or_else(|| {
                    tracing::warn!("Reference to unknown ivar: ${name}");
                    None
                })?;
                self.eval(referenced, ctx)?
            }
        };

        for adj in &expr.adjustments {
            acc = adj.apply(acc);
        }
        Some(acc)
    }
}

fn strip_ivar_prefix(name: &str) -> &str {
    name.strip_prefix('$').unwrap_or(name)
}

// ---------------------------------------------------------------------------
// Parsed (expression-form) types
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct ButtonAdviceExpr {
    pub id: String,
    pub x: IExpr,
    pub y: IExpr,
    pub dx: IExpr,
    pub dy: IExpr,
    pub as_3d_index: Option<i32>,
    pub unpressed: Option<i32>,
    pub pressed: Option<i32>,
    pub middle: Option<i32>,
}

impl Default for IExpr {
    fn default() -> Self {
        Self::zero()
    }
}

#[derive(Debug, Clone)]
pub struct VertexExpr {
    pub x: IExpr,
    pub y: IExpr,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
    pub u: Option<UvExpr>,
    pub v: Option<UvExpr>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Polygon {
    pub i0: u32,
    pub i1: u32,
    pub i2: u32,
}

#[derive(Debug, Default)]
pub struct GeometryNormalExpr {
    pub texture: String,
    pub texture_pack_dx: IExpr,
    pub texture_pack_dy: IExpr,
    pub blend_mode: String,
    pub bilinear_filtering: bool,

    pub vertices: Vec<VertexExpr>,
    pub polygons: Vec<Polygon>,
}

#[derive(Debug, Default)]
pub struct GeometryTiledExpr {
    pub jpg_name: String,
    pub dimensions: [IExpr; 2],
    pub chunk_dimensions: [IExpr; 2],
    pub bilinear_filtering: bool,
}

#[derive(Debug)]
pub enum GeometryExpr {
    Normal(GeometryNormalExpr),
    Tiled(GeometryTiledExpr),
}

#[derive(Debug, Default)]
pub struct RectExpr {
    pub x: IExpr,
    pub y: IExpr,
    pub w: IExpr,
    pub h: IExpr,
}

#[derive(Debug, Default)]
pub struct WindowBase {
    pub name: String,
    pub dx: IExpr,
    pub dy: IExpr,
    pub render_dx: IExpr,
    pub render_dy: IExpr,
    pub reload_on_mode_switch: bool,

    pub button_advices: HashMap<String, ButtonAdviceExpr>,
    pub geometries: Vec<GeometryExpr>,
    pub ivars: IVars,
    pub within_regions: Vec<RectExpr>,
}

// ---------------------------------------------------------------------------
// Resolved (layout-form) types
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct ButtonAdvice {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub dx: i32,
    pub dy: i32,
    pub as_3d_index: Option<i32>,
    pub unpressed: Option<i32>,
    pub pressed: Option<i32>,
    pub middle: Option<i32>,
}

#[derive(Debug, Default, Clone, Copy)]
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

#[derive(Debug, Default, Clone)]
pub struct GeometryNormal {
    pub texture: String,
    pub texture_pack_dx: i32,
    pub texture_pack_dy: i32,
    pub blend_mode: String,
    pub bilinear_filtering: bool,

    pub vertices: Vec<Vertex>,
    pub polygons: Vec<Polygon>,
}

#[derive(Debug, Default, Clone)]
pub struct GeometryTiled {
    pub jpg_name: String,
    pub dimensions: [i32; 2],
    pub chunk_dimensions: [i32; 2],
    pub bilinear_filtering: bool,
}

#[derive(Debug, Clone)]
pub enum Geometry {
    Normal(GeometryNormal),
    Tiled(GeometryTiled),
}

#[derive(Debug, Default, Clone)]
pub struct WindowBaseLayout {
    pub dx: i32,
    pub dy: i32,
    pub render_dx: i32,
    pub render_dy: i32,

    pub button_advices: HashMap<String, ButtonAdvice>,
    pub geometries: Vec<Geometry>,
    pub within_regions: Vec<Rect>,
}

// ---------------------------------------------------------------------------
// Layout resolution
// ---------------------------------------------------------------------------

impl WindowBase {
    /// Resolve every expression in this window base against `ctx` (typically
    /// `{"screen_dx": logical_width, "screen_dy": logical_height}`), producing
    /// a fully concrete layout.
    pub fn layout(&self, ctx: &WindowLayoutContext) -> WindowBaseLayout {
        let eval = |e: &IExpr| self.ivars.eval(e, ctx).unwrap_or(0);
        let eval_uv = |uv: &Option<UvExpr>| match uv {
            None => None,
            Some(UvExpr::Auto) => Some(AUTO_UV),
            Some(UvExpr::Expr(e)) => Some(eval(e)),
        };

        let button_advices = self
            .button_advices
            .iter()
            .map(|(id, a)| {
                (
                    id.clone(),
                    ButtonAdvice {
                        id: a.id.clone(),
                        x: eval(&a.x),
                        y: eval(&a.y),
                        dx: eval(&a.dx),
                        dy: eval(&a.dy),
                        as_3d_index: a.as_3d_index,
                        unpressed: a.unpressed,
                        pressed: a.pressed,
                        middle: a.middle,
                    },
                )
            })
            .collect();

        let geometries = self
            .geometries
            .iter()
            .map(|g| match g {
                GeometryExpr::Normal(n) => Geometry::Normal(GeometryNormal {
                    texture: n.texture.clone(),
                    texture_pack_dx: eval(&n.texture_pack_dx),
                    texture_pack_dy: eval(&n.texture_pack_dy),
                    blend_mode: n.blend_mode.clone(),
                    bilinear_filtering: n.bilinear_filtering,
                    vertices: n
                        .vertices
                        .iter()
                        .map(|v| Vertex {
                            x: eval(&v.x),
                            y: eval(&v.y),
                            r: v.r,
                            g: v.g,
                            b: v.b,
                            a: v.a,
                            u: eval_uv(&v.u),
                            v: eval_uv(&v.v),
                        })
                        .collect(),
                    polygons: n.polygons.clone(),
                }),
                GeometryExpr::Tiled(t) => Geometry::Tiled(GeometryTiled {
                    jpg_name: t.jpg_name.clone(),
                    dimensions: [eval(&t.dimensions[0]), eval(&t.dimensions[1])],
                    chunk_dimensions: [eval(&t.chunk_dimensions[0]), eval(&t.chunk_dimensions[1])],
                    bilinear_filtering: t.bilinear_filtering,
                }),
            })
            .collect();

        let within_regions = self
            .within_regions
            .iter()
            .map(|r| {
                Rect::new(
                    IVec2::new(eval(&r.x), eval(&r.y)),
                    IVec2::new(eval(&r.w), eval(&r.h)),
                )
            })
            .collect();

        WindowBaseLayout {
            dx: eval(&self.dx),
            dy: eval(&self.dy),
            render_dx: eval(&self.render_dx),
            render_dy: eval(&self.render_dy),
            button_advices,
            geometries,
            within_regions,
        }
    }
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

impl From<ConfigLines> for WindowBase {
    fn from(value: ConfigLines) -> Self {
        let mut window_base = WindowBase::default();

        #[derive(Debug, Default)]
        enum State {
            #[default]
            None,
            Normal(GeometryNormalExpr),
            Tiled(GeometryTiledExpr),
        }

        impl State {
            fn flush(&mut self, window_base: &mut WindowBase) {
                match std::mem::take(self) {
                    State::None => {}
                    State::Normal(geometry) => {
                        window_base.geometries.push(GeometryExpr::Normal(geometry));
                    }
                    State::Tiled(geometry) => {
                        window_base.geometries.push(GeometryExpr::Tiled(geometry));
                    }
                }
            }
        }

        let mut state = State::None;

        for line in value.into_iter() {
            let opens_new_block = matches!(
                line.key.as_str(),
                "WINDOW_BASE_GEOMETRY" | "WINDOW_BASE_GEOMETRY_TILED"
            );
            let is_geometry_field = line.key.starts_with("GEOMETRY_");

            if !is_geometry_field && !opens_new_block {
                state.flush(&mut window_base);
            }

            match (&mut state, line.key.as_str()) {
                (_, "WINDOW_BASE") => window_base.name = line.string(0),
                (_, "WINDOW_BASE_DX") => window_base.dx = parse_expr_or_zero(&line, 0),
                (_, "WINDOW_BASE_DY") => window_base.dy = parse_expr_or_zero(&line, 0),
                (_, "WINDOW_BASE_RENDER_DX") => {
                    window_base.render_dx = parse_expr_or_zero(&line, 0);
                }
                (_, "WINDOW_BASE_RENDER_DY") => {
                    window_base.render_dy = parse_expr_or_zero(&line, 0);
                }
                (_, "WINDOW_BASE_RELOAD_ON_MODE_SWITCH") => {
                    window_base.reload_on_mode_switch = line.param::<bool>(0);
                }

                (_, "DEFINE_BUTTON_ADVICE") => {
                    let advice = ButtonAdviceExpr {
                        id: line.string(0),
                        x: parse_expr_or_zero(&line, 1),
                        y: parse_expr_or_zero(&line, 2),
                        dx: parse_expr_or_zero(&line, 3),
                        dy: parse_expr_or_zero(&line, 4),
                        as_3d_index: line.maybe_param(5),
                        unpressed: line.maybe_param(6),
                        pressed: line.maybe_param(7),
                        middle: line.maybe_param(8),
                    };
                    window_base.button_advices.insert(advice.id.clone(), advice);
                }

                (_, "DEFINE_USER_IVAR") => {
                    window_base.ivars.define(&line);
                }

                (_, "MODIFY_USER_IVAR") => {
                    window_base.ivars.modify(&line);
                }

                (_, "DEFINE_WITHIN_REGION") => {
                    let kind = line.string(0);
                    if kind != "RECT" {
                        tracing::warn!("Unsupported DEFINE_WITHIN_REGION kind: {kind}");
                    } else {
                        window_base.within_regions.push(RectExpr {
                            x: parse_expr_or_zero(&line, 1),
                            y: parse_expr_or_zero(&line, 2),
                            w: parse_expr_or_zero(&line, 3),
                            h: parse_expr_or_zero(&line, 4),
                        });
                    }
                }

                (_, "WINDOW_BASE_GEOMETRY") => {
                    state.flush(&mut window_base);
                    state = State::Normal(GeometryNormalExpr::default());
                }
                (_, "WINDOW_BASE_GEOMETRY_TILED") => {
                    state.flush(&mut window_base);
                    state = State::Tiled(GeometryTiledExpr::default());
                }

                (State::Normal(g), "GEOMETRY_TEXTURE") => g.texture = line.string(0),
                (State::Normal(g), "GEOMETRY_TEXTURE_PACK_DX") => {
                    g.texture_pack_dx = parse_expr_or_zero(&line, 0);
                }
                (State::Normal(g), "GEOMETRY_TEXTURE_PACK_DY") => {
                    g.texture_pack_dy = parse_expr_or_zero(&line, 0);
                }
                (State::Normal(g), "GEOMETRY_BLEND_MODE") => g.blend_mode = line.string(0),
                (State::Normal(g), "GEOMETRY_BILINEAR_FILTERING") => {
                    g.bilinear_filtering = line.string(0).eq_ignore_ascii_case("on");
                }
                (State::Normal(_), "GEOMETRY_VERTICES") => {}
                (State::Normal(g), "GEOMETRY_VERTEX") => {
                    let x = parse_expr_or_zero(&line, 0);
                    let y = parse_expr_or_zero(&line, 1);
                    let r = line.param::<f32>(2);
                    let g_ = line.param::<f32>(3);
                    let b = line.param::<f32>(4);
                    let a = line.param::<f32>(5);
                    let (u, v) = if line.params().len() >= 8 {
                        (
                            Some(parse_uv_param(&line, 6)),
                            Some(parse_uv_param(&line, 7)),
                        )
                    } else {
                        (None, None)
                    };
                    g.vertices.push(VertexExpr {
                        x,
                        y,
                        r,
                        g: g_,
                        b,
                        a,
                        u,
                        v,
                    });
                }
                (State::Normal(_), "GEOMETRY_POLYGONS") => {}
                (State::Normal(g), "GEOMETRY_POLYGON") => {
                    g.polygons.push(Polygon {
                        i0: line.param::<i32>(0) as u32,
                        i1: line.param::<i32>(1) as u32,
                        i2: line.param::<i32>(2) as u32,
                    });
                }

                (State::Tiled(g), "GEOMETRY_JPG_NAME") => g.jpg_name = line.string(0),
                (State::Tiled(g), "GEOMETRY_JPG_DIMENSIONS") => {
                    g.dimensions = [parse_expr_or_zero(&line, 0), parse_expr_or_zero(&line, 1)];
                }
                (State::Tiled(g), "GEOMETRY_CHUNK_DIMENSIONS") => {
                    g.chunk_dimensions =
                        [parse_expr_or_zero(&line, 0), parse_expr_or_zero(&line, 1)];
                }
                (State::Tiled(g), "GEOMETRY_BILINEAR_FILTERING") => {
                    g.bilinear_filtering = line.string(0).eq_ignore_ascii_case("on");
                }

                _ => {
                    tracing::warn!("Unknown key {} for state: {state:?}", line.key);
                }
            }
        }

        state.flush(&mut window_base);

        window_base
    }
}

fn parse_expr_or_zero(line: &ConfigLine, index: usize) -> IExpr {
    parse_expr_param(line, index).unwrap_or_else(IExpr::zero)
}

fn parse_expr_param(line: &ConfigLine, index: usize) -> Option<IExpr> {
    let token = line.params().get(index)?;
    parse_expr_token(token)
}

fn parse_uv_param(line: &ConfigLine, index: usize) -> UvExpr {
    let Some(token) = line.params().get(index) else {
        return UvExpr::Expr(IExpr::zero());
    };
    if let ConfigToken::String(s) = token
        && s.eq_ignore_ascii_case("AUTO")
    {
        return UvExpr::Auto;
    }
    UvExpr::Expr(parse_expr_token(token).unwrap_or_else(IExpr::zero))
}

fn parse_expr_token(token: &ConfigToken) -> Option<IExpr> {
    match token {
        ConfigToken::Number(n) => Some(IExpr::number(*n)),
        ConfigToken::Float(f) => Some(IExpr::number(*f as i32)),
        ConfigToken::String(s) => parse_expr_string(s),
    }
}

/// Parse text like `%screen_dx`, `$ivar`, `42`, `%screen_dx,-198`, or
/// `%screen_dx,/2,-64`. The first segment yields the atom; each remaining
/// comma-separated segment carries an operator prefix and an integer.
fn parse_expr_string(text: &str) -> Option<IExpr> {
    let mut parts = text.split(',');
    let head = parts.next()?.trim();
    let atom = parse_atom(head)?;

    let mut adjustments = Vec::new();
    for segment in parts {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        let (op, rest) = segment.split_at(1);
        let operand = rest.trim().parse::<i32>().ok()?;
        adjustments.push(AdjOp::parse(op, operand)?);
    }

    Some(IExpr { atom, adjustments })
}

fn parse_atom(text: &str) -> Option<Atom> {
    if let Some(name) = text.strip_prefix('%') {
        return Some(Atom::Var(name.to_string()));
    }
    if let Some(name) = text.strip_prefix('$') {
        return Some(Atom::IVar(name.to_string()));
    }
    text.parse::<i32>().ok().map(Atom::Number)
}
