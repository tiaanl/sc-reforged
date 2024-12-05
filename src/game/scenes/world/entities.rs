use crate::{
    engine::prelude::*,
    game::{
        mesh_renderer::{MeshItem, MeshList, MeshRenderer},
        model::{Model, NodeIndex},
    },
};

/// Represents an object inside the game world.
#[derive(Debug)]
pub struct Entity {
    pub transform: Transform,
    pub model: Handle<Model>,
}

impl Entity {
    pub fn new(transform: Transform, model: Handle<Model>) -> Self {
        Self { transform, model }
    }
}

pub struct Entities {
    asset_manager: AssetManager,
    pub model_renderer: MeshRenderer,
    pub entities: Vec<Entity>,
}

impl Entities {
    pub fn new(
        asset_manager: AssetManager,
        renderer: &Renderer,
        shaders: &mut Shaders,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let models = MeshRenderer::new(
            asset_manager.clone(),
            renderer,
            shaders,
            camera_bind_group_layout,
        );
        Self {
            asset_manager,
            model_renderer: models,
            entities: vec![],
        }
    }

    pub fn spawn(&mut self, entity: Entity) {
        self.entities.push(entity);
    }

    pub fn render(
        &self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        camera_bind_group: &wgpu::BindGroup,
    ) {
        // Build a list of all the meshes that needs rendering.
        let mut list = MeshList::default();
        for entity in self.entities.iter() {
            let Some(model) = self.asset_manager.get(entity.model) else {
                continue;
            };

            for mesh in model.meshes.iter() {
                // Calculate the model's global transform.
                let mut node_id = mesh.node_id;
                let mut transform = entity.transform.to_mat4();
                while node_id != NodeIndex::MAX {
                    let node = &model.nodes[node_id];
                    transform = transform * node.transform.to_mat4();
                    node_id = node.parent;
                }

                list.meshes.push(MeshItem {
                    transform,
                    mesh: mesh.mesh,
                });
            }
        }

        self.model_renderer.render_multiple(
            renderer,
            encoder,
            output,
            camera_bind_group,
            list,
            wgpu::LoadOp::Load,
        );
    }
}
