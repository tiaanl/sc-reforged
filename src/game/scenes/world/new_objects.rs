use std::path::PathBuf;

use crate::{
    engine::{
        prelude::*,
        storage::{Handle, Storage},
    },
    game::{config::ObjectType, math::BoundingSphere, model::Model, models::models},
};

enum ObjectData {
    Scenery {
        model: Handle<Model>,
        model_des: Handle<Model>,
    },
}

pub struct NewObject {
    pub transform: Transform,
    pub bounding_sphere: BoundingSphere,
    data: ObjectData,
}

#[derive(Default)]
pub struct NewObjects {
    pub objects: Storage<NewObject>,
}

impl NewObjects {
    pub fn spawn(
        &mut self,
        transform: Transform,
        radius: f32,
        object_type: ObjectType,
        name: &str,
        _title: &str,
    ) -> Result<Handle<NewObject>, AssetError> {
        let bounding_sphere = BoundingSphere {
            center: transform.translation,
            radius,
        };

        match object_type {
            ObjectType::Scenery => {
                let model = models().load_model(
                    name,
                    PathBuf::from("models")
                        .join(name)
                        .join(name)
                        .with_extension("smf"),
                )?;

                let model_des = models().load_model(
                    name,
                    PathBuf::from("models")
                        .join(name)
                        .join(format!("{name}_des"))
                        .with_extension("smf"),
                )?;

                Ok(self.objects.insert(NewObject {
                    transform,
                    bounding_sphere,
                    data: ObjectData::Scenery { model, model_des },
                }))
            }
            _ => Err(AssetError::Decode(PathBuf::default())),
        }
    }
}
