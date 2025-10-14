use std::path::PathBuf;

use crate::{
    engine::{
        prelude::*,
        storage::{Handle, Storage},
    },
    game::{
        config::ObjectType, math::BoundingSphere, model::Model, models::models,
        scenes::world::render::RenderStore,
    },
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

impl NewObject {
    pub fn model_to_render(&self) -> Option<Handle<Model>> {
        Some(match self.data {
            ObjectData::Scenery { model, .. } => model,
        })
    }
}

#[derive(Default)]
pub struct NewObjects {
    /// A list for all objects iun the world.
    pub objects: Storage<NewObject>,

    /// Keep a list of handles to try and load.
    models_to_prepare: Vec<Handle<Model>>,
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
            ObjectType::Scenery
            | ObjectType::SceneryAlarm
            | ObjectType::SceneryBush
            | ObjectType::SceneryFragile
            | ObjectType::SceneryLit
            | ObjectType::SceneryShadowed
            | ObjectType::SceneryStripLight
            | ObjectType::SceneryTree
            | ObjectType::Structure
            | ObjectType::StructureArmGate
            | ObjectType::StructureBridge
            | ObjectType::StructureBuggable
            | ObjectType::StructureBuilding
            | ObjectType::StructureBuildingGateController
            | ObjectType::StructureDoubleGate
            | ObjectType::StructureFence
            | ObjectType::StructureGuardTower
            | ObjectType::StructureLadderSlant0_11
            | ObjectType::StructureLadderSlant0_14
            | ObjectType::StructureLadderSlant0_16
            | ObjectType::StructureLadderSlant0_2
            | ObjectType::StructureLadderSlant0_3
            | ObjectType::StructureLadderSlant0_5
            | ObjectType::StructureLadderSlant0_6
            | ObjectType::StructureLadderSlant0_9
            | ObjectType::StructureLadderSlant2_2
            | ObjectType::StructureLadderSlant2_4
            | ObjectType::StructureLadderSlant2_5
            | ObjectType::StructureLocker
            | ObjectType::StructureSAM
            | ObjectType::StructureSingleGate
            | ObjectType::StructureSlideBridge
            | ObjectType::StructureSlideBridgeController
            | ObjectType::StructureSlideDoor
            | ObjectType::StructureStripLightController
            | ObjectType::StructureSwingDoor
            | ObjectType::StructureTent
            | ObjectType::StructureWall => {
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

                self.models_to_prepare.push(model);
                self.models_to_prepare.push(model_des);

                Ok(self.objects.insert(NewObject {
                    transform,
                    bounding_sphere,
                    data: ObjectData::Scenery { model, model_des },
                }))
            }
            _ => Err(AssetError::Decode(PathBuf::default())),
        }
    }

    #[inline]
    pub fn get(&self, handle: Handle<NewObject>) -> Option<&NewObject> {
        self.objects.get(handle)
    }

    /// Take all models that were used during `spawn` and prepare them to be rendered.
    pub fn prepare_models(&mut self, render_store: &mut RenderStore) {
        if self.models_to_prepare.is_empty() {
            return;
        }

        let mut models_to_prepare = Vec::default();
        std::mem::swap(&mut self.models_to_prepare, &mut models_to_prepare);

        tracing::info!("Preparing {} models for the GPU.", models_to_prepare.len());

        for model_handle in models_to_prepare {
            if let Err(err) = render_store.get_or_create_render_model(model_handle) {
                tracing::warn!("Could not prepare model! ({err})");
            }
        }
    }
}
