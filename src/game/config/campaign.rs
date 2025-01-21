use glam::{Vec2, Vec3};

use crate::game::asset_loader::AssetError;

use super::ConfigFile;

#[derive(Default)]
pub struct ViewInitial {
    pub from: Vec2,
    pub to: Vec2,
}

/*
TOD_DATA_SUN_X		0.0		0.0		-0.04		-0.08		-.04		-2.0		-1.08		-1.31		-1.54		0.17		0.17		0.17		0.17		0.17		0.17		1.0		1.9		1.6		1.3		1.1		0.51		0.05		0.0		0.0
TOD_DATA_SUN_Y		0.00		0.00		0.03		0.06		0.06		0.06		0.06		0.06		0.06		0.06		0.06		0.06		0.06		0.06		0.06		0.06		0.06		0.06		0.06		0.06		0.06		0.06		0.0		0.0
TOD_DATA_SUN_Z		-1.0		-1.0		-1.0		-1.0		-1.0		-1.0		-1.0		-1.0		-1.0		-1.0		-1.0		-1.0		-1.0		-1.0		-1.0		-1.0		-1.0		-1.0		-1.0		-1.0		-1.0		-1.0		-1.0		-1.0

TOD_DATA_SUN_R		0.4		0.4		0.44		0.44		0.44		0.55		0.66		0.70		0.79		0.83		0.87		0.93		1.00		1.00		1.00		1.00		1.00		1.00		1.00		0.90		0.74		0.440		0.44		0.4
TOD_DATA_SUN_G		0.4		0.4		0.44		0.44		0.44		0.63		0.77		0.83		0.90		0.95		1.00		1.00		1.00		1.00		1.00		1.00		0.97		0.9		0.88		0.77		0.61		0.440		0.44		0.4
TOD_DATA_SUN_B		0.414		0.414		0.454		0.454		0.454		0.62		0.81		0.88		0.96		0.98		1.00		1.00		1.00		1.00		1.00		0.94		0.87		0.82		0.77		0.63		0.49		0.454		0.454		0.414

TOD_DATA_FOG_D		18000.0	18000.0	18000.0	16000.0	15000.0	14000.0	13000.0	13000.0	13000.0	13500.0	14500.0	16000.0	17000.0	17000.0	17000.0	17000.0	17000.0	17000.0	17000.0	17000.0	17000.0	17000.0	17000.0	17000.0
TOD_DATA_FOG_N		0.325		0.325		0.325		0.325		0.325		0.325		0.325		0.325		0.45		0.44		0.43		0.41		0.4		0.375		0.35		0.325		0.325		0.325		0.325		0.325		0.325		0.325		0.325		0.325
TOD_DATA_FOG_R		0.067		0.115		0.156		0.156		0.267		0.35		0.41		0.47		0.56		0.48		0.4		0.41		0.42		0.41		0.4		0.4		0.4		0.4		0.4		0.3		0.156		0.156		0.12		0.067
TOD_DATA_FOG_G		0.022		0.07		0.111		0.111		0.2		0.35		0.365		0.38		0.53		0.48		0.42		0.42		0.42		0.4		0.42		0.4		0.4		0.4		0.4		0.3		0.111		0.111		0.06		0.022
TOD_DATA_FOG_B		0.022		0.05		0.089		0.089		0.2		0.35		0.32		0.29		0.51		0.47		0.44		0.44		0.44		0.4		0.44		0.4		0.4		0.4		0.4		0.3		0.089		0.089		0.04		0.022
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

impl TryFrom<String> for Campaign {
    type Error = AssetError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut campaign = Campaign::default();

        let mut config = ConfigFile::new(&value);

        macro_rules! tod {
            ($params:expr,$s1:ident) => {{
                for i in 0..24 {
                    if let Some(value) = $params.get(i + 1) {
                        campaign.time_of_day[i].$s1 = value.parse().unwrap_or(0.0);
                    }
                }
            }};
            ($params:expr,$s1:ident.$s2:ident) => {{
                for i in 0..24 {
                    if let Some(value) = $params.get(i + 1) {
                        campaign.time_of_day[i].$s1.$s2 = value.parse().unwrap_or(0.0);
                    }
                }
            }};
        }

        while let Some(current) = config.current() {
            match current[0] {
                "SPECIFY_VIEW_INITIAL" => {
                    campaign.view_initial.from.x = current[1].parse().unwrap();
                    campaign.view_initial.from.y = current[2].parse().unwrap();
                    campaign.view_initial.to.x = current[3].parse().unwrap();
                    campaign.view_initial.to.y = current[4].parse().unwrap();
                }
                "SPECIFY_MTF" => {
                    campaign.mtf_name = Some(current[1].into());
                }

                "TOD_DATA_SUN_X" => tod!(current, sun_dir.x),
                "TOD_DATA_SUN_Y" => tod!(current, sun_dir.y),
                "TOD_DATA_SUN_Z" => tod!(current, sun_dir.z),
                "TOD_DATA_SUN_R" => tod!(current, sun_color.x),
                "TOD_DATA_SUN_G" => tod!(current, sun_color.y),
                "TOD_DATA_SUN_B" => tod!(current, sun_color.z),
                "TOD_DATA_FOG_D" => tod!(current, fog_distance),
                "TOD_DATA_FOG_N" => tod!(current, fog_near_fraction),
                "TOD_DATA_FOG_R" => tod!(current, fog_color.x),
                "TOD_DATA_FOG_G" => tod!(current, fog_color.y),
                "TOD_DATA_FOG_B" => tod!(current, fog_color.z),

                _ => {}
            }
            config.next();
        }

        Ok(campaign)
    }
}
