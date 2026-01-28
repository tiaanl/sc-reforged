use bevy_ecs::prelude::*;

use crate::{
    engine::{storage::Handle, transform::Transform},
    game::{
        model::Model,
        scenes::world::{
            extract::{ModelToRender, RenderSnapshot},
            sim_world::{ComputedCamera, DynamicBvh, StaticBvh, ecs::ActiveCamera},
            systems::world_interaction::WorldInteraction,
        },
    },
};

#[allow(clippy::too_many_arguments)]
pub fn extract_model_snapshot(
    mut snapshot: ResMut<RenderSnapshot>,
    models: Query<(Entity, &Transform, &Handle<Model>)>,
    models_to_prepare: Query<&Handle<Model>, Added<Handle<Model>>>,
    static_bvh: Res<StaticBvh>,
    dynamic_bvh: Res<DynamicBvh>,
    computed_camera: Single<&ComputedCamera, With<ActiveCamera>>,
    world_interaction: Res<WorldInteraction>,
    mut visible_objects_cache: Local<Vec<Entity>>,
) {
    snapshot.models.models_to_prepare.clear();

    {
        models_to_prepare.iter().for_each(|&model_handle| {
            snapshot.models.models_to_prepare.push(model_handle);
        });
    }

    snapshot.models.models.clear();

    {
        visible_objects_cache.clear();

        static_bvh.objects_in_frustum(&computed_camera.frustum, &mut visible_objects_cache);
        dynamic_bvh.query_frustum(&computed_camera.frustum, &mut visible_objects_cache);

        for (entity, transform, model_handle) in models.iter_many(&visible_objects_cache) {
            snapshot.models.models.push(ModelToRender {
                model: *model_handle,
                transform: transform.to_mat4(),
                highlighted: world_interaction
                    .selected_entity
                    .map(|e| e == entity)
                    .unwrap_or(false),
            });
        }
    }
}
