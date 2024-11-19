use glam::{Mat3, Mat4, Quat, Vec3};
use shadow_company_tools::smf;
use tracing::warn;

use crate::engine::{
    arena::{Arena, Handle},
    assets::Assets,
    mesh::{GpuMesh, RenderPassMeshExt},
    renderer::{RenderPipelineConfig, Renderer},
    shaders::Shaders,
};

use super::{bounding_boxes::BoundingBoxes, textures::Textures};

#[derive(Debug)]
pub struct Mesh {
    pub mesh: GpuMesh,
    pub texture: Handle<wgpu::BindGroup>,
}

#[derive(Debug)]
struct BoundingBox {
    min: Vec3,
    max: Vec3,
}

#[derive(Debug)]
pub struct Node {
    pub position: Vec3,
    pub rotation: Quat,
    pub meshes: Vec<Mesh>,
    pub children: Vec<Node>,
    pub bounding_boxes: Vec<BoundingBox>,
}

#[derive(Debug)]
pub struct Model {
    pub nodes: Vec<Node>,
}

#[derive(Debug)]
pub struct RenderInfo {
    position: Vec3,
    rotation: Vec3,
    handle: Handle<Model>,
}

impl RenderInfo {
    pub fn new(position: Vec3, rotation: Vec3, handle: Handle<Model>) -> Self {
        Self {
            position,
            rotation,
            handle,
        }
    }
}

pub struct Models {
    render_pipeline: wgpu::RenderPipeline,

    textures: Textures,
    models: Arena<Model>,
}

impl Models {
    pub fn new(renderer: &Renderer, shaders: &mut Shaders) -> Self {
        let shader_module = shaders.create_shader(
            renderer,
            "model_renderer",
            include_str!("model.wgsl"),
            "model.wgsl",
        );

        let render_pipeline = renderer.create_render_pipeline(
            RenderPipelineConfig::<crate::engine::mesh::Vertex>::new(
                "model_renderer",
                &shader_module,
            )
            .bind_group_layout(renderer.uniform_bind_group_layout())
            .bind_group_layout(renderer.uniform_bind_group_layout())
            .bind_group_layout(renderer.texture_bind_group_layout()),
        );

        let textures = Textures::default();

        let models = Arena::default();

        Self {
            render_pipeline,
            textures,
            models,
        }
    }

    pub fn insert(
        &mut self,
        renderer: &Renderer,
        assets: &Assets,
        smf: &smf::Model,
    ) -> Handle<Model> {
        let model = self
            .smf_to_model(renderer, assets, smf)
            .expect("Could not load model! FIX THIS");
        self.models.insert(model)
    }

    pub fn render_multiple(
        &self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        textures: &Textures,
        camera_bind_group: &wgpu::BindGroup,
        batch: &[RenderInfo],
        bounding_boxes: &mut BoundingBoxes,
    ) {
        let mut render_pass = Self::create_render_pass(renderer, encoder, output);
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        batch.iter().for_each(|render_info| {
            let Some(model) = self.models.get(&render_info.handle) else {
                return;
            };

            model.nodes.iter().for_each(|node| {
                self.render_node(
                    renderer,
                    &mut render_pass,
                    node,
                    render_info.position,
                    Quat::IDENTITY,
                    bounding_boxes,
                );
            });
        });
    }

    fn render_node(
        &self,
        renderer: &Renderer,
        render_pass: &mut wgpu::RenderPass<'_>,
        node: &Node,
        position: Vec3,
        rotation: Quat,
        bounding_boxes: &mut BoundingBoxes,
    ) {
        {
            let model_matrix = Mat4::from_rotation_translation(rotation, position);
            let data = model_matrix.to_cols_array_2d();
            let buffer = renderer.create_uniform_buffer("model_matrix_buffer", data);
            let model_bind_group = renderer.create_uniform_bind_group("model_matrix", &buffer);

            render_pass.set_bind_group(1, &model_bind_group, &[]);

            node.meshes.iter().for_each(|mesh| {
                if let Some(texture_bind_group) = self.textures.get(&mesh.texture) {
                    render_pass.set_bind_group(2, texture_bind_group, &[]);
                    render_pass.draw_mesh(&mesh.mesh);
                } else {
                    warn!("Could not find model texture!");
                }
            });

            for b in node.bounding_boxes.iter() {
                bounding_boxes.insert(position, rotation, b.min, b.max);
            }
        }

        for node in node.children.iter() {
            self.render_node(
                renderer,
                render_pass,
                node,
                position + node.position,
                rotation * node.rotation,
                bounding_boxes,
            )
        }
    }

    fn create_render_pass<'encoder>(
        renderer: &Renderer,
        encoder: &'encoder mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
    ) -> wgpu::RenderPass<'encoder> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("model_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: renderer
                .render_pass_depth_stencil_attachment(wgpu::LoadOp::Load),
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }
}

impl Models {
    fn smf_to_model(
        &mut self,
        renderer: &Renderer,
        assets: &Assets,
        smf: &smf::Model,
    ) -> Option<Model> {
        let nodes = self.children_of(renderer, assets, &smf.nodes, "<root>");
        if nodes.len() != 1 {
            panic!("Only a single root node is supported!");
        }

        Some(Model { nodes })
    }

    fn children_of(
        &mut self,
        renderer: &Renderer,
        assets: &Assets,
        nodes: &[smf::Node],
        parent_name: &str,
    ) -> Vec<Node> {
        // 180-degree rotation around the Y-axis
        let rotation_y_180 = Quat::from_rotation_y(std::f32::consts::PI);
        // -90-degree rotation around the X-axis
        let rotation_x_neg_90 = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);

        // Combine the rotations
        let transform_quat = rotation_x_neg_90 * rotation_y_180;

        // Find all the child nodes.
        nodes
            .iter()
            .filter(|node| node.parent_name == parent_name)
            .map(|node| Node {
                position: node.position,
                rotation: transform_quat * node.rotation,
                meshes: node
                    .meshes
                    .iter()
                    .map(|mesh| self.smf_mesh_to_mesh(renderer, assets, mesh))
                    .collect(),
                children: self.children_of(renderer, assets, nodes, &node.name),
                bounding_boxes: node
                    .collision_boxes
                    .iter()
                    .map(|b| BoundingBox {
                        min: b.min,
                        max: b.max,
                    })
                    .collect(),
            })
            .collect()
    }

    fn smf_mesh_to_mesh(&mut self, renderer: &Renderer, assets: &Assets, mesh: &smf::Mesh) -> Mesh {
        let vertices = mesh
            .vertices
            .iter()
            .map(|v| crate::engine::mesh::Vertex {
                position: v.position,
                normal: v.normal,
                tex_coord: v.tex_coord,
            })
            .collect::<Vec<_>>();

        let indices = mesh
            .faces
            .iter()
            .flat_map(|i| i.indices)
            .collect::<Vec<_>>();

        let gpu_mesh = crate::engine::mesh::Mesh { vertices, indices }.to_gpu(renderer);

        let texture_path = std::path::PathBuf::from("textures")
            .join("shared")
            .join(&mesh.texture_name);

        let texture_handle = self
            .textures
            .get_by_path_or_insert(texture_path, |path| {
                let image = match assets.load_bmp(path) {
                    Ok(image) => image,
                    Err(e) => {
                        warn!("Could not load texture! {} {:?}", path.display(), e);
                        return None;
                    }
                };
                let texture_view = renderer.create_texture_view(path.to_str().unwrap(), image);

                // TODO: Reuse a sampler.
                let sampler = renderer.create_sampler(
                    "texture_sampler",
                    wgpu::AddressMode::ClampToEdge,
                    wgpu::FilterMode::Linear,
                    wgpu::FilterMode::Linear,
                );

                let bind_group = renderer.create_texture_bind_group(
                    path.to_str().unwrap(),
                    &texture_view,
                    &sampler,
                );

                Some(bind_group)
            })
            .expect("Could not create texture");

        Mesh {
            mesh: gpu_mesh,
            texture: texture_handle,
        }
    }
}
