use bevy_ecs::prelude::*;

use crate::{
    engine::{storage::Handle, transform::Transform},
    game::{
        model::Model,
        scenes::world::{
            render::{ModelRenderFlags, ModelToRender},
            sim_world::{
                ComputedCamera, DynamicBvh, StaticBvh,
                ecs::{ActiveCamera, Snapshots},
            },
        },
    },
};

pub fn extract_model_snapshot(
    mut snapshots: ResMut<Snapshots>,
    models: Query<(Entity, &Transform, &Handle<Model>)>,
    models_to_prepare: Query<&Handle<Model>, Added<Handle<Model>>>,
    static_bvh: Res<StaticBvh>,
    dynamic_bvh: Res<DynamicBvh<Entity>>,
    computed_camera: Single<&ComputedCamera, With<ActiveCamera>>,
    mut visible_objects_cache: Local<Vec<Entity>>,
) {
    snapshots.model_render_snapshot.models_to_prepare.clear();

    {
        models_to_prepare.iter().for_each(|&model_handle| {
            snapshots
                .model_render_snapshot
                .models_to_prepare
                .push(model_handle);
        });
    }

    snapshots.model_render_snapshot.models.clear();

    {
        visible_objects_cache.clear();

        static_bvh.objects_in_frustum(&computed_camera.frustum, &mut visible_objects_cache);
        dynamic_bvh.query_frustum(&computed_camera.frustum, &mut visible_objects_cache);

        for (_entity, transform, model_handle) in models.iter_many(&visible_objects_cache) {
            let flags = ModelRenderFlags::empty();

            // TODO: Mark selected objects.
            /*
            flags.set(
                ModelRenderFlags::HIGHLIGHTED,
                selected_objects.contains(&entity),
            );
            */

            snapshots.model_render_snapshot.models.push(ModelToRender {
                model: *model_handle,
                transform: transform.to_mat4(),
                flags,
            });
        }
    }
}
