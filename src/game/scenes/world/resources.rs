use bevy_ecs::prelude as ecs;

use crate::game::renderer::ModelRenderer;

#[derive(ecs::Resource)]
pub struct DeltaTime(pub f32);

#[derive(ecs::Resource)]
pub struct SelectedEntity(pub Option<ecs::Entity>);

#[derive(ecs::Resource)]
pub struct ModelRendererResource(pub ModelRenderer);
