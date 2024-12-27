use crate::game::asset_loader::AssetError;

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
        }
    }
}

impl TryFrom<String> for TerrainMapping {
    type Error = AssetError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut config = ConfigFile::new(&value);

        let mut result = TerrainMapping::default();

        while let Some(params) = config.current() {
            if params[0] == "SET" {
                match params[1] {
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
                }
            }
            config.next();
        }

        Ok(result)
    }
}
