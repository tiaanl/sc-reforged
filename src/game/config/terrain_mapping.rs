// Note from terrain_mapping.txt:
//
// `altitude_map_height_base`, `min_alt_grad` and `max_alt_grad` define how an altitude table is
// generated. Of the `256` altitude levels, the first is `0.0` centimeters tall, the next is the
// `previous + (rand() % (max_alt_grad - min_alt_grad) + min_alt_grad) * altitude_map_height_base`.

use glam::Vec2;

use crate::game::config::parser::ConfigLines;

#[derive(Debug)]
pub struct TerrainMapping {
    pub fully_textured_map: bool,
    pub texture_map_base_name: String,
    pub terrain_textures_dx: i32,
    pub terrain_textures_dy: i32,
    // Must be power of 2.
    pub map_dx: i32,
    // Must be power of 2.
    pub map_dy: i32,
    pub water_level: i32,
    pub nominal_edge_size: f32,
    // Defaults to 1 and indicates the number of averaging smooth passes to
    // be performed on the map after it is generated.
    pub map_smooth_passes: i32,
    // Defaults to 3 and indicates the number of texture constraint passes to be performed.
    // Additional constraint passes can be executed from the Terrain Editor window.
    pub constraint_passes: i32,
    pub altitude_map_height_base: f32,
    pub min_alt_grad: i32,
    pub max_alt_grad: i32,

    // The water modulate textures should be in the image processor directory as .bmp files.
    pub water_modulate_textures: [String; 2],
    // Set the W1 and W2 Modulators; these define how fast and in what direction the water modulate
    // textures are modulated.
    pub w1_modulator: Vec2,
    pub w2_modulator: Vec2,

    // The following define how the water fades based on depth. `trans_high` and `trans_low` are the
    // bounding alpha values (from 0..=255) and the `trans_depth` is the distance over which they
    // are interpolated between.
    pub water_trans_depth: f32,
    pub water_trans_high: i32,
    pub water_trans_low: i32,

    // Wind dx,dy defaults to 10.0 10.0, and is the direction the texture shift moves on the water
    // (as well as other stuff)
    pub wind_direction: Vec2,

    // Set the water period (ms), wavelength and amplitude (cm)
    pub water_period: f32,
    pub water_wavelength: f32,
    pub water_amplitude: f32,
}

impl Default for TerrainMapping {
    fn default() -> Self {
        Self {
            fully_textured_map: false,
            texture_map_base_name: String::new(),
            terrain_textures_dx: 0,
            terrain_textures_dy: 0,
            map_dx: 0,
            map_dy: 0,
            water_level: 0,
            nominal_edge_size: 16.0,
            map_smooth_passes: 1,
            constraint_passes: 3,
            altitude_map_height_base: 6.0,
            min_alt_grad: 1,
            max_alt_grad: 4,

            water_modulate_textures: [String::new(), String::new()],

            w1_modulator: Vec2::ZERO,
            w2_modulator: Vec2::ZERO,

            water_trans_depth: 0.0,
            water_trans_high: 0,
            water_trans_low: 0,

            wind_direction: Vec2::new(10.0, 10.0),

            water_period: 0.0,
            water_wavelength: 0.0,
            water_amplitude: 0.0,
        }
    }
}

impl From<ConfigLines> for TerrainMapping {
    fn from(value: ConfigLines) -> Self {
        let mut terrain_mapping = Self::default();

        for line in value.iter() {
            match line.key.as_str() {
                "SET" => match line.string(0).as_str() {
                    "fully_textured_map" => terrain_mapping.fully_textured_map = line.param(1),
                    "texture_map_base_name" => {
                        terrain_mapping.texture_map_base_name = line.param(1)
                    }
                    "terrain_textures_dx" => terrain_mapping.terrain_textures_dx = line.param(1),
                    "terrain_textures_dy" => terrain_mapping.terrain_textures_dy = line.param(1),
                    "map_dx" => terrain_mapping.map_dx = line.param(1),
                    "map_dy" => terrain_mapping.map_dy = line.param(1),
                    "water_level" => terrain_mapping.water_level = line.param(1),
                    "nominal_edge_size" => terrain_mapping.nominal_edge_size = line.param(1),
                    "map_smooth_passes" => terrain_mapping.map_smooth_passes = line.param(1),
                    "constraint_passes" => terrain_mapping.constraint_passes = line.param(1),
                    "altitude_map_height_base" => {
                        terrain_mapping.altitude_map_height_base = line.param(1)
                    }
                    "min_alt_grad" => terrain_mapping.min_alt_grad = line.param(1),
                    "max_alt_grad" => terrain_mapping.max_alt_grad = line.param(1),

                    _ => {
                        tracing::warn!("Unknown TerrainMapping SET key: {}", line.string(0))
                    }
                },

                "LOAD_FULLY_TEXTURED_MAP_SET" => {
                    // Note from terrain_mapping.txt:
                    //
                    // Next, define the textures that we want to be loaded (out of the TextureMaps
                    // directory.)  These TextureMaps should be 128x128 .BMP files. (starts counting
                    // at 1, as error.bmp is always texture index 0)
                    //
                    // LOAD_FULLY_TEXTURED_MAP_SET not_important_but_needs_a_field
                }

                "SET_WATER_MODULATE_TEXTURES" => {
                    terrain_mapping.water_modulate_textures = [line.param(0), line.param(1)];
                }

                "SET_W1_MODULATORS" => {
                    terrain_mapping.w1_modulator = Vec2::new(line.param(0), line.param(1));
                }

                "SET_W2_MODULATORS" => {
                    terrain_mapping.w2_modulator = Vec2::new(line.param(0), line.param(1))
                }

                "WATER_TRANS_DEPTH" => terrain_mapping.water_trans_depth = line.param(1),
                "WATER_TRANS_HIGH" => terrain_mapping.water_trans_high = line.param(1),
                "WATER_TRANS_LOW" => terrain_mapping.water_trans_low = line.param(1),

                "SET_WIND_DIRECTION" => {
                    terrain_mapping.wind_direction = Vec2::new(line.param(0), line.param(1));
                }

                "SET_WATER_PERIOD" => terrain_mapping.water_period = line.param(0),
                "SET_WATER_WAVELENGTH" => terrain_mapping.water_wavelength = line.param(0),
                "SET_WATER_AMPLITUDE" => terrain_mapping.water_amplitude = line.param(0),

                "LOAD_TEXTURE" | "SET_WATER_TEXTURE" => {
                    // TODO: Document why these are ignored.
                }

                _ => tracing::warn!("Unknown TerrainMapping key: {}", line.key),
            }
        }

        terrain_mapping
    }
}
