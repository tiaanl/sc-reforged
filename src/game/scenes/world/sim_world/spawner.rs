use bevy_ecs::{entity::Entity, world::World};

use crate::{
    engine::{assets::AssetError, prelude::Transform},
    game::{
        config::ObjectType,
        models::{ModelName, models},
    },
};

pub struct Spawner<'world> {
    world: &'world mut World,
}

impl<'world> Spawner<'world> {
    pub fn new(world: &'world mut World) -> Self {
        Self { world }
    }

    pub fn spawn(
        &mut self,
        transform: Transform,
        object_type: ObjectType,
        name: &str,
        _title: &str,
    ) -> Result<Entity, AssetError> {
        match object_type {
            _ => {
                let (model_handle, _model) = models().load_model(ModelName::Object(name.into()))?;

                Ok(self.world.spawn((transform, model_handle)).id())
            }
        }
    }
}
