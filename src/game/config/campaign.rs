use glam::{Vec2, Vec3};

use crate::{
    engine::assets::AssetError,
    game::config::parser::{ConfigLine, ConfigLines},
};

#[derive(Default)]
pub struct ViewInitial {
    pub from: Vec2,
    pub to: Vec2,
}

impl From<ConfigLine> for ViewInitial {
    fn from(value: ConfigLine) -> Self {
        Self {
            from: Vec2::new(value.param(0), value.param(1)),
            to: Vec2::new(value.param(2), value.param(3)),
        }
    }
}

/*
TOD_DATA_SUN_X   0.00     0.00    -0.04    -0.08    -0.04    -2.00    -1.08    -1.31    -1.54     0.17    0.17    0.17    0.17     0.17     0.17    1.00    1.90    1.60    1.30    1.10    0.51    0.05    0.00    0.00
TOD_DATA_SUN_Y   0.00     0.00     0.03     0.06     0.06     0.06     0.06     0.06     0.06     0.06    0.06    0.06    0.06     0.06     0.06    0.06    0.06    0.06    0.06    0.06    0.06    0.06    0.00    0.00
TOD_DATA_SUN_Z  -1.00    -1.00    -1.00    -1.00    -1.00    -1.00    -1.00    -1.00    -1.00    -1.00   -1.00   -1.00   -1.00    -1.00    -1.00   -1.00   -1.00   -1.00   -1.00   -1.00   -1.00   -1.00   -1.00   -1.00

TOD_DATA_SUN_R   0.40     0.40     0.44     0.44     0.44     0.55     0.66     0.70     0.79     0.83    0.87    0.93    1.00     1.00     1.00    1.00    1.00    1.00    1.00    0.90    0.74    0.44    0.44    0.40
TOD_DATA_SUN_G   0.40     0.40     0.44     0.44     0.44     0.63     0.77     0.83     0.90     0.95    1.00    1.00    1.00     1.00     1.00    1.00    0.97    0.90    0.88    0.77    0.61    0.44    0.44    0.40
TOD_DATA_SUN_B   0.414    0.414    0.454    0.454    0.454    0.62     0.81     0.88     0.96     0.98    1.00    1.00    1.00     1.00     1.00    0.94    0.87    0.82    0.77    0.63    0.49    0.454   0.454   0.414

TOD_DATA_FOG_D 18000.00 18000.00 18000.00 16000.00 15000.00 14000.00 13000.00 13000.00 13000.00 13500.00 14500.00 16000.00 17000.00 17000.00 17000.00 17000.00 17000.00 17000.00 17000.00 17000.00 17000.00 17000.00 17000.00 17000.00
TOD_DATA_FOG_N   0.325    0.325    0.325    0.325    0.325    0.325    0.325    0.325    0.450    0.440   0.430   0.410   0.400    0.375    0.350   0.325   0.325   0.325   0.325   0.325   0.325   0.325   0.325   0.325
TOD_DATA_FOG_R   0.067    0.115    0.156    0.156    0.267    0.350    0.410    0.470    0.560    0.480   0.400   0.410   0.420    0.410    0.400   0.400   0.400   0.400   0.400   0.300   0.156   0.156   0.120   0.067
TOD_DATA_FOG_G   0.022    0.070    0.111    0.111    0.200    0.350    0.365    0.380    0.530    0.480   0.420   0.420   0.420    0.400    0.420   0.400   0.400   0.400   0.400   0.300   0.111   0.111   0.060   0.022
TOD_DATA_FOG_B   0.022    0.050    0.089    0.089    0.200    0.350    0.320    0.290    0.510    0.470   0.440   0.440   0.440    0.400    0.440   0.400   0.400   0.400   0.400   0.300   0.089   0.089   0.040   0.022
*/

#[derive(Default)]
pub struct TimeOfDayEntry {
    pub sun_dir: Vec3,
    pub sun_color: Vec3,
    pub fog_distance: f32,
    pub fog_near_fraction: f32,
    pub fog_color: Vec3,
}

#[derive(Default)]
pub struct Campaign {
    pub view_initial: ViewInitial,
    pub mtf_name: Option<String>,

    pub time_of_day: [TimeOfDayEntry; 24],
}

impl From<ConfigLines> for Campaign {
    fn from(value: ConfigLines) -> Self {
        let mut campaign = Self::default();

        for line in value.into_iter() {
            match line.key.as_str() {
                "SPECIFY_VIEW_INITIAL" => campaign.view_initial = line.into(),
                "SPECIFY_MTF" => campaign.mtf_name = line.maybe_param(0),
                "TOD_DATA_SUN_X" => {
                    (0..24).for_each(|i| campaign.time_of_day[i].sun_dir.x = line.param(i))
                }
                "TOD_DATA_SUN_Y" => {
                    (0..24).for_each(|i| campaign.time_of_day[i].sun_dir.y = line.param(i))
                }
                "TOD_DATA_SUN_Z" => {
                    (0..24).for_each(|i| campaign.time_of_day[i].sun_dir.z = line.param(i))
                }
                "TOD_DATA_SUN_R" => {
                    (0..24).for_each(|i| campaign.time_of_day[i].sun_color.x = line.param(i))
                }
                "TOD_DATA_SUN_G" => {
                    (0..24).for_each(|i| campaign.time_of_day[i].sun_color.y = line.param(i))
                }
                "TOD_DATA_SUN_B" => {
                    (0..24).for_each(|i| campaign.time_of_day[i].sun_color.z = line.param(i))
                }
                "TOD_DATA_FOG_D" => {
                    (0..24).for_each(|i| campaign.time_of_day[i].fog_distance = line.param(i))
                }
                "TOD_DATA_FOG_N" => {
                    (0..24).for_each(|i| campaign.time_of_day[i].fog_near_fraction = line.param(i))
                }
                "TOD_DATA_FOG_R" => {
                    (0..24).for_each(|i| campaign.time_of_day[i].fog_color.x = line.param(i))
                }
                "TOD_DATA_FOG_G" => {
                    (0..24).for_each(|i| campaign.time_of_day[i].fog_color.y = line.param(i))
                }
                "TOD_DATA_FOG_B" => {
                    (0..24).for_each(|i| campaign.time_of_day[i].fog_color.z = line.param(i))
                }

                _ => tracing::warn!("Invalid key for Campaign: {}", line.key),
            }
        }

        campaign
    }
}
