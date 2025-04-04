use glam::UVec2;

use crate::{
    Asset,
    engine::assets::resources::{ResourceLoadContext, ResourceType},
};

use super::mesh_renderer::BlendMode;

pub struct Image {
    pub size: UVec2,
    pub blend_mode: BlendMode,
    pub data: image::RgbaImage,
}

impl Image {
    pub fn from_rgba(data: image::RgbaImage, blend_mode: BlendMode) -> Self {
        Self {
            size: UVec2::new(data.width(), data.height()),
            blend_mode,
            data,
        }
    }
}

impl Asset for Image {}

impl ResourceType for Image {
    fn from_data(data: Vec<u8>, context: &ResourceLoadContext) -> Result<Self, ()> {
        let is_color_keyd = context
            .path()
            .file_name()
            .map(|n| n.to_string_lossy().contains("_ck"))
            .unwrap_or(false);

        let ext = context.path().extension().unwrap().to_ascii_lowercase();
        if ext == "bmp" {
            let bmp = shadow_company_tools::images::load_bmp_file(
                &mut std::io::Cursor::new(data),
                is_color_keyd,
            )
            .map_err(|_| ())?;

            let raw =
                if let Ok(raw_data) = context.load_direct(context.path().with_extension("raw")) {
                    Some(
                        shadow_company_tools::images::load_raw_file(
                            &mut std::io::Cursor::new(raw_data),
                            bmp.width(),
                            bmp.height(),
                        )
                        .map_err(|_| ())?,
                    )
                } else {
                    None
                };

            return Ok(if is_color_keyd {
                Image::from_rgba(
                    image::DynamicImage::from(bmp).into_rgba8(),
                    BlendMode::ColorKeyed,
                )
            } else if let Some(raw) = raw {
                Image::from_rgba(
                    shadow_company_tools::images::combine_bmp_and_raw(&bmp, &raw),
                    BlendMode::Alpha,
                )
            } else {
                Image::from_rgba(
                    image::DynamicImage::from(bmp).into_rgba8(),
                    BlendMode::Opaque,
                )
            });
        }

        Err(())
    }
}
