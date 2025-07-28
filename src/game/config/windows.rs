use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct ButtonAdvice {
    id: String,
    x: i32,
    y: i32,
    dx: i32,
    dy: i32,
    as_3d_index: Option<i32>,
    unpressed: Option<i32>,
    pressed: Option<i32>,
    middle: Option<i32>,
}

impl ButtonAdvice {
    fn from_params(params: &[&str]) -> Self {
        if params.len() < 2 {
            Self {
                id: params[0].to_string(),
                ..Default::default()
            }
        } else {
            Self {
                id: params[0].parse().unwrap(),
                x: params[1].parse().unwrap(),
                y: params[2].parse().unwrap(),
                dx: params[3].parse().unwrap(),
                dy: params[4].parse().unwrap(),
                as_3d_index: (params.len() > 5).then(|| params[5].parse().unwrap()),
                unpressed: (params.len() > 6).then(|| params[6].parse().unwrap()),
                pressed: (params.len() > 7).then(|| params[7].parse().unwrap()),
                middle: (params.len() > 8).then(|| params[8].parse().unwrap()),
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct Vertex {
    x_pos: f32,
    y_pos: f32,
    r: f32,
    g: f32,
    b: f32,
    tu: Option<f32>,
    tv: Option<f32>,
}

impl Vertex {
    fn from_params(params: &[&str]) -> Self {
        Self {
            x_pos: params[0].parse().unwrap(),
            y_pos: params[1].parse().unwrap(),
            r: params[2].parse().unwrap(),
            g: params[3].parse().unwrap(),
            b: params[4].parse().unwrap(),
            tu: (params.len() > 5).then(|| params[5].parse().unwrap()),
            tv: (params.len() > 6).then(|| params[6].parse().unwrap()),
        }
    }
}

#[derive(Debug, Default)]
pub struct Polygon {
    f1: u32,
    f2: u32,
    f3: u32,
}

impl Polygon {
    fn from_params(params: &[&str]) -> Self {
        Self {
            f1: params[0].parse().unwrap(),
            f2: params[0].parse().unwrap(),
            f3: params[0].parse().unwrap(),
        }
    }
}

#[derive(Debug, Default)]
pub struct Geometry {
    texture: String,
    texture_pack_dx: u32,
    texture_pack_dy: u32,
    bilinear_filtering: bool,
    blend_mode: String,

    vertex_count: u32,
    vertices: Vec<Vertex>,

    polygon_count: u32,
    polygons: Vec<Polygon>,
}

#[derive(Debug, Default)]
pub struct GeometryTiled {
    name: String,
    dimensions: [u32; 2],
    chunk_dimensions: [u32; 2],
}

#[derive(Debug, Default)]
pub struct WindowBase {
    name: String,
    dx: u32,
    dy: u32,
    render_dx: u32,
    render_dy: u32,

    button_advices: Vec<ButtonAdvice>,
    geometries: Vec<Geometry>,
    geometries_tiled: Vec<GeometryTiled>,
    ivars: HashMap<String, i32>,
}

/*
pub fn read_window_base_file(data: &str) -> WindowBase {
    let mut config = ConfigFile::new(data);

    #[derive(Debug)]
    enum State {
        WindowBase(WindowBase),
        Geometry(WindowBase, Geometry),
        GeometryTiled(WindowBase, GeometryTiled),
    }

    impl State {
        fn with_window_base(&mut self) -> &mut WindowBase {
            match self {
                Self::WindowBase(window_base) => window_base,
                _ => panic!("Required state WindowBase, but found {self:?}"),
            }
        }

        fn with_geometry(&mut self) -> &mut Geometry {
            match self {
                Self::Geometry(_, geometry) => geometry,
                _ => panic!("Required state Geometry, but found {self:?}"),
            }
        }

        fn pop_geometry(self) -> WindowBase {
            let Self::Geometry(mut window_base, geometry) = self else {
                panic!("Trying to pop geometry in wrong state. {self:?}");
            };

            window_base.geometries.push(geometry);
            window_base
        }

        fn with_geometry_tiled(&mut self) -> &mut GeometryTiled {
            match self {
                Self::GeometryTiled(_, geometry_tiled) => geometry_tiled,
                _ => panic!("Required state GeometryTiled, but found {self:?}"),
            }
        }

        fn pop_geometry_tiled(self) -> WindowBase {
            let Self::GeometryTiled(mut window_base, geometry_tiled) = self else {
                panic!("Trying to pop geometry_tiled in wrong state. {self:?}");
            };

            window_base.geometries_tiled.push(geometry_tiled);
            window_base
        }
    }

    let mut state = State::WindowBase(WindowBase::default());

    while let Some(current) = config.current() {
        match current[0] {
            "WINDOW_BASE" => state.with_window_base().name = current[1].to_string(),
            "WINDOW_BASE_DX" => state.with_window_base().dx = current[1].parse().unwrap(),
            "WINDOW_BASE_DY" => state.with_window_base().dy = current[1].parse().unwrap(),
            "WINDOW_BASE_RENDER_DX" => {
                state.with_window_base().render_dx = current[1].parse().unwrap()
            }
            "WINDOW_BASE_RENDER_DY" => {
                state.with_window_base().render_dy = current[1].parse().unwrap()
            }

            "DEFINE_BUTTON_ADVICE" => state
                .with_window_base()
                .button_advices
                .push(ButtonAdvice::from_params(&current[1..])),

            "DEFINE_USER_IVAR" => {
                state
                    .with_window_base()
                    .ivars
                    .insert(current[1].to_string(), current[2].parse().unwrap());
            }

            "WINDOW_BASE_GEOMETRY" => match state {
                State::WindowBase(window_base) => {
                    state = State::Geometry(window_base, Geometry::default())
                }
                State::Geometry(..) => {
                    state = State::Geometry(state.pop_geometry(), Geometry::default());
                }
                State::GeometryTiled(..) => {
                    state = State::Geometry(state.pop_geometry_tiled(), Geometry::default());
                }
            },

            "GEOMETRY_TEXTURE" => state.with_geometry().texture = current[1].to_string(),
            "GEOMETRY_TEXTURE_PACK_DX" => {
                state.with_geometry().texture_pack_dx = current[1].parse().unwrap()
            }
            "GEOMETRY_TEXTURE_PACK_DY" => {
                state.with_geometry().texture_pack_dy = current[1].parse().unwrap()
            }
            "GEOMETRY_BILINEAR_FILTERING" => {
                state.with_geometry().bilinear_filtering = current[1] == "on";
            }
            "GEOMETRY_BLEND_MODE" => state.with_geometry().blend_mode = current[1].to_string(),

            "GEOMETRY_VERTICES" => state.with_geometry().vertex_count = current[1].parse().unwrap(),

            "GEOMETRY_VERTEX" => state
                .with_geometry()
                .vertices
                .push(Vertex::from_params(&current[1..])),

            "GEOMETRY_POLYGONS" => {
                state.with_geometry().polygon_count = current[1].parse().unwrap()
            }

            "GEOMETRY_POLYGON" => state
                .with_geometry()
                .polygons
                .push(Polygon::from_params(&current[1..])),

            "WINDOW_BASE_GEOMETRY_TILED" => match state {
                State::WindowBase(ref mut window_base) => {
                    let window_base = std::mem::take(window_base);
                    state = State::GeometryTiled(window_base, GeometryTiled::default());
                }
                State::Geometry(..) => {
                    state = State::GeometryTiled(state.pop_geometry(), GeometryTiled::default());
                }
                State::GeometryTiled(..) => {
                    state =
                        State::GeometryTiled(state.pop_geometry_tiled(), GeometryTiled::default());
                }
            },

            "GEOMETRY_JPG_NAME" => state.with_geometry_tiled().name = current[1].to_string(),
            "GEOMETRY_JPG_DIMENSIONS" => {
                state.with_geometry_tiled().dimensions =
                    [current[1].parse().unwrap(), current[2].parse().unwrap()]
            }
            "GEOMETRY_CHUNK_DIMENSIONS" => {
                state.with_geometry_tiled().chunk_dimensions =
                    [current[1].parse().unwrap(), current[2].parse().unwrap()]
            }
            _ => panic!("Invalid config line. {current:?}"),
        }
        config.next();
    }

    loop {
        match state {
            State::WindowBase(window_base) => return window_base,
            State::Geometry(..) => {
                state = State::WindowBase(state.pop_geometry());
            }
            State::GeometryTiled(..) => {
                state = State::WindowBase(state.pop_geometry_tiled());
            }
        }
    }
}
*/
