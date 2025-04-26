use glam::{Vec3, Vec4};

use crate::game::{assets::Config, config::ConfigFile};

#[derive(Debug)]
pub struct Object {
    // OBJECT Scenery_Strip_Light AlScLt-Runway "AlScLt-Runway"
    pub typ: String,
    pub name: String,
    pub title: String,

    // OBJECT_POSITION 25413.458984 26909.134766 1202.765137
    pub position: Vec3,
    // OBJECT_ROTATION 0.000000 0.000000 0.000000
    pub rotation: Vec3,
    // OBJECT_ID 1 230
    pub id: [i32; 2],
    // OBJECT_MTF_CONFIG 1500 900 120 350 100.000000 100.000000 1500.000000 850.000000
    pub config: (i32, i32, i32, i32, f32, f32, f32, f32),
}

impl Object {
    fn from_params(params: &[&str]) -> Self {
        Self {
            typ: params[1].to_string(),
            name: params[2].to_string(),
            title: params[2].to_string(),
            position: Vec3::ZERO,
            rotation: Vec3::ZERO,
            id: [0, 0],
            config: (0, 0, 0, 0, 0.0, 0.0, 0.0, 0.0),
        }
    }
}

#[derive(Debug, Default)]
pub struct Fog {
    pub start: f32,
    pub end: f32,
    pub color: Vec3,
}

impl Fog {
    fn from_params(params: &[&str]) -> Self {
        // Defaults are from training_final.mtf
        // GAME_CONFIG_FOG_ENABLED 0.419306 0.418611 0.437222 0.000000 13300.000000
        // TODO: There are defaults in the .exe.
        Self {
            start: params[4].parse().unwrap_or(0.0),
            end: params[5].parse().unwrap_or(13_300.0),
            color: Vec3::new(
                params[1].parse().unwrap_or(0.419306),
                params[2].parse().unwrap_or(0.418611),
                params[3].parse().unwrap_or(0.437222),
            ),
        }
    }
}

#[derive(Debug, Default)]
pub struct Mtf {
    pub time_of_day: [u32; 2],
    pub game_config_fog_enabled: Fog,
    pub inventory: Vec<Object>,
    pub objects: Vec<Object>,
}

impl Config for Mtf {
    fn from_string(str: &str) -> Result<Self, crate::engine::assets::AssetError> {
        let mut config = ConfigFile::new(str);

        #[derive(Debug)]
        enum State {
            None,
            ObjectInventory(Object),
            Object(Object),
        }

        let mut mtf = Mtf::default();
        let mut state = State::None;

        while let Some(current) = config.current() {
            match current[0] {
                "GAME_STATE_TIME_OF_DAY" => {
                    mtf.time_of_day = [current[1].parse().unwrap(), current[2].parse().unwrap()];
                }
                "GAME_CONFIG_FOG_ENABLED" => {
                    mtf.game_config_fog_enabled = Fog::from_params(current);
                }
                "OBJECT_INVENTORY" => {
                    match state {
                        State::None => {}
                        State::ObjectInventory(old) => {
                            mtf.inventory.push(old);
                        }
                        State::Object(old) => {
                            mtf.objects.push(old);
                        }
                    }
                    state = State::ObjectInventory(Object::from_params(current));
                }
                "OBJECT" => {
                    match state {
                        State::None => {}
                        State::ObjectInventory(old) => {
                            mtf.inventory.push(old);
                        }
                        State::Object(old) => {
                            mtf.objects.push(old);
                        }
                    }
                    state = State::Object(Object::from_params(current));
                }
                "OBJECT_POSITION" => {
                    let position = Vec3::new(
                        current[1].parse().unwrap_or(0.0),
                        current[2].parse().unwrap_or(0.0),
                        current[3].parse().unwrap_or(0.0),
                    );
                    match state {
                        State::None => panic!("No object selected!"),
                        State::ObjectInventory(ref mut obj) => obj.position = position,
                        State::Object(ref mut obj) => obj.position = position,
                    }
                }
                "OBJECT_ROTATION" => {
                    let rotation = Vec3::new(
                        current[1].parse().unwrap_or(0.0),
                        current[2].parse().unwrap_or(0.0),
                        current[3].parse().unwrap_or(0.0),
                    );
                    match state {
                        State::None => panic!("No object selected!"),
                        State::ObjectInventory(ref mut obj) => obj.rotation = rotation,
                        State::Object(ref mut obj) => obj.rotation = rotation,
                    }
                }
                "OBJECT_ID" => {
                    let id = [current[1].parse().unwrap(), current[2].parse().unwrap()];
                    match state {
                        State::None => panic!("No object selected!"),
                        State::ObjectInventory(ref mut obj) => obj.id = id,
                        State::Object(ref mut obj) => obj.id = id,
                    }
                }
                "OBJECT_MTF_CONFIG" => {
                    // Just skip this for now.
                }
                _ => panic!("Invalid MTF entry: {:?}", config.current()),
            }
            config.next();
        }

        match state {
            State::None => {}
            State::ObjectInventory(object) => mtf.inventory.push(object),
            State::Object(object) => mtf.objects.push(object),
        }

        Ok(mtf)
    }
}
