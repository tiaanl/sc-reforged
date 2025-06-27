use glam::Vec2;

use crate::{engine::assets::AssetError, game::assets::Config};

use super::ConfigFile;

/// `map_dx` and `map_dy` must be powers of two.
/// `altitude_map_height_base`, `min_alt_grad` and `max_alt_grad` define how an altitude table is
/// generated.  Of the 256 altitude levels, the first is 0.0 centimeters tall, the next is
/// `previous + (rand() % (max_alt_grad - min_alt_grad) + min_alt_grad) * altitude_map_height_base`.
#[derive(Debug)]
pub struct TerrainMapping {
    pub fully_textured_map: bool,
    pub texture_map_base_name: String,
    pub terrain_textures_dx: i32,
    pub terrain_textures_dy: i32,
    pub map_dx: f32,
    pub map_dy: f32,
    pub water_level: f32,
    pub nominal_edge_size: f32,
    /// Indicates the number of averaging smooth passes to be performed on the map after it is
    /// generated.
    pub map_smooth_passes: i32,
    /// indicates the number of texture constraint passes to be performed.  Additional constraint
    /// passes can be executed from the Terrain Editor window.
    pub constraint_passes: i32,
    pub altitude_map_height_base: f32,
    pub min_alt_grad: i32,
    pub max_alt_grad: i32,

    // Water and other
    pub water_modulate_textures: Vec<String>,
    pub w1_modulators: Vec2,
    pub w2_modulators: Vec2,
    pub water_trans_depth: f32,
    pub water_trans_high: u8,
    pub water_trans_low: u8,
    pub wind_direction: Vec2,
    pub water_period: f32,
    pub water_wavelength: f32,
    pub water_amplitude: f32,
}

impl Default for TerrainMapping {
    fn default() -> Self {
        Self {
            fully_textured_map: false,
            texture_map_base_name: "".to_string(),
            terrain_textures_dx: 8,
            terrain_textures_dy: 8,
            map_dx: 16.0,
            map_dy: 16.0,
            water_level: 0.0,
            nominal_edge_size: 16.0,
            map_smooth_passes: 1,
            constraint_passes: 3,
            altitude_map_height_base: 6.0,
            min_alt_grad: 1,
            max_alt_grad: 4,

            // TODO: These are probably not the right defaults.
            water_modulate_textures: vec![],
            w1_modulators: Vec2::ZERO,
            w2_modulators: Vec2::ZERO,
            water_trans_depth: 0.0,
            water_trans_high: 255,
            water_trans_low: 0,
            wind_direction: Vec2::ZERO,
            water_period: 0.0,
            water_wavelength: 0.0,
            water_amplitude: 0.0,
        }
    }
}

impl Config for TerrainMapping {
    fn from_string(str: &str) -> Result<Self, AssetError> {
        let mut config = ConfigFile::new(str);

        let mut result = TerrainMapping::default();

        while let Some(params) = config.current() {
            match params[0] {
                "SET" => match params[1] {
                    "map_dx" => result.map_dx = params[2].parse().unwrap(),
                    "map_dy" => result.map_dy = params[2].parse().unwrap(),
                    "water_level" => result.water_level = params[2].parse().unwrap(),
                    "nominal_edge_size" => result.nominal_edge_size = params[2].parse().unwrap(),
                    "map_smooth_passes" => result.map_smooth_passes = params[2].parse().unwrap(),
                    "constraint_passes" => result.constraint_passes = params[2].parse().unwrap(),
                    "altitude_map_height_base" => {
                        result.altitude_map_height_base = params[2].parse().unwrap()
                    }
                    "min_alt_grad" => result.min_alt_grad = params[2].parse().unwrap(),
                    "max_alt_grad" => result.max_alt_grad = params[2].parse().unwrap(),

                    "fully_textured_map" => result.fully_textured_map = params[2].parse().unwrap(),
                    "texture_map_base_name" => {
                        result.texture_map_base_name = params[2].parse().unwrap()
                    }
                    "terrain_textures_dx" => {
                        result.terrain_textures_dx = params[2].parse().unwrap()
                    }
                    "terrain_textures_dy" => {
                        result.terrain_textures_dy = params[2].parse().unwrap()
                    }
                    _ => panic!("Invalid parameter {}", params[1]),
                },

                "LOAD_FULLY_TEXTURED_MAP_SET" => {
                    // ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
                    // ; next, define the textures that we want to be loaded (out of the
                    // ; TextureMaps directory.)  These TextureMaps should be 128x128 .BMP
                    // ; files. (starts counting at 1, as error.bmp is always texture index 0)
                    //
                    // LOAD_FULLY_TEXTURED_MAP_SET      not_important_but_needs_a_field
                }

                "SET_WATER_MODULATE_TEXTURES" => {
                    if params.len() > 1 {
                        result.water_modulate_textures.push(params[1].to_string());
                        if params.len() > 2 {
                            result.water_modulate_textures.push(params[2].to_string());
                        }
                    }
                }
                "SET_W1_MODULATORS" => {
                    result.w1_modulators.x = params[1].parse().unwrap();
                    result.w1_modulators.y = params[2].parse().unwrap();
                }
                "SET_W2_MODULATORS" => {
                    result.w2_modulators.x = params[1].parse().unwrap();
                    result.w2_modulators.y = params[2].parse().unwrap();
                }
                "WATER_TRANS_DEPTH" => result.water_trans_depth = params[1].parse().unwrap(),
                "WATER_TRANS_HIGH" => result.water_trans_high = params[1].parse().unwrap(),
                "WATER_TRANS_LOW" => result.water_trans_low = params[1].parse().unwrap(),
                "SET_WIND_DIRECTION" => {
                    result.wind_direction.x = params[1].parse().unwrap();
                    result.wind_direction.y = params[2].parse().unwrap();
                }
                "SET_WATER_PERIOD" => result.water_period = params[1].parse().unwrap(),
                "SET_WATER_WAVELENGTH" => result.water_wavelength = params[1].parse().unwrap(),
                "SET_WATER_AMPLITUDE" => result.water_amplitude = params[1].parse().unwrap(),

                "WATER_EXCEPTION" => {
                    // TODO: build a list of patches that get different water levels.
                }

                "LOAD_TEXTURE" | "SET_WATER_TEXTURE" => {
                    // TODO: Looks like only the training campaign has this set. Same for
                    //       LOAD_TEXTURE, which looks to be the same texture (water.bmp) and just
                    //       happens to be the only campaign without water...
                }

                e => panic!("Unexpected parameter {e}"),
            }
            config.next();
        }

        Ok(result)
    }
}
