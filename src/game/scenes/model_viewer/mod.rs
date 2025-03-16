use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use glam::Vec4;
use shadow_company_tools::{bmf, smf};
use wgpu::util::DeviceExt;

use crate::engine::gizmos::{GizmoVertex, GizmosRenderer};
use crate::engine::prelude::*;
use crate::game::animation::Track;
use crate::game::asset_loader::{AssetError, AssetLoader};
use crate::game::camera;

pub struct ModelViewer {
    asset_loader: Arc<AssetLoader>,

    model: Option<Model>,
    animation: Option<Animation>,

    node_data_bind_group_layout: wgpu::BindGroupLayout,
    material_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,

    camera: camera::Camera,
    camera_controller: camera::ArcBallCameraController,
    gpu_camera: camera::GpuCamera,

    depth_texture: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,

    models: DirNode,
    model_to_load: Option<PathBuf>,

    animations: DirNode,
    animation_to_load: Option<PathBuf>,

    gizmos_renderer: GizmosRenderer,

    time: f32,
}

impl ModelViewer {
    pub fn new(renderer: &Renderer, asset_loader: Arc<AssetLoader>) -> Result<Self, AssetError> {
        let node_data_bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("node_data_bind_group_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let material_bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("material_bind_group_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let sampler = renderer.create_sampler(
            "model_viewer_sampler",
            wgpu::AddressMode::Repeat,
            wgpu::FilterMode::Linear,
            wgpu::FilterMode::Linear,
        );

        let camera = camera::Camera::new(
            Vec3::new(0.0, -100.0, 0.0),
            Quat::IDENTITY,
            45.0,
            1.0,
            1.0,
            10_000.0,
        );
        let camera_controller = camera::ArcBallCameraController::new(0.5);
        let gpu_camera = camera::GpuCamera::new(renderer);

        let mut shaders = Shaders::new();
        shaders.add_module(include_str!("../../common/camera.wgsl"), "camera.wgsl");
        shaders.add_module(
            include_str!("../world/environment.wgsl"),
            "environment.wgsl",
        );

        let depth_texture = DepthBuffer::create_texture(&renderer.device, &renderer.surface_config);

        let module = shaders.create_shader(
            renderer,
            "model_viewer",
            include_str!("model_viewer.wgsl"),
            "model_viewer.wgsl",
            HashMap::default(),
        );

        let pipeline_layout =
            renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("model_viewer_pipeline_layout"),
                    bind_group_layouts: &[
                        &material_bind_group_layout,
                        &gpu_camera.bind_group_layout,
                        &node_data_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });

        let pipeline = renderer
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("model_viewer_render_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[Vertex::layout()],
                },
                primitive: wgpu::PrimitiveState {
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DepthBuffer::FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: renderer.surface_config.format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            });

        let mut models = DirNode::default();
        for path in asset_loader
            .enum_dir(&PathBuf::from("models"))?
            .iter()
            .filter(|p| p.extension().map(|e| e == "smf").unwrap_or(false))
        {
            models.insert(path.clone());
        }

        let mut animations = DirNode::default();
        for path in asset_loader
            .enum_dir(&PathBuf::from("motions"))?
            .iter()
            .filter(|p| p.extension().map(|e| e == "bmf").unwrap_or(false))
        {
            animations.insert(path.clone());
        }

        let gizmos_renderer = GizmosRenderer::new(renderer, &gpu_camera.bind_group_layout);

        let model = {
            let path = PathBuf::from("models")
                .join("alan-crow01")
                .join("alan-crow01.smf");
            let smf = asset_loader.load_smf_direct(&path)?;

            Model::from_smf(
                renderer,
                &asset_loader,
                &node_data_bind_group_layout,
                &material_bind_group_layout,
                &sampler,
                &smf,
            )
            .expect("Could not load model.")
        };

        let animation = None;

        // let animation = {
        //     let path = PathBuf::from("motions").join("crow_flight_cycle.bmf");
        //     let bmf = asset_loader.load_bmf_direct(path)?;

        //     Animation::from_bmf(&bmf)?
        // };

        // let animation = {
        //     let mut animation = Animation::default();
        //     let rotations = animation.rotations.entry(55).or_default();
        //     rotations.set_key_frame(0.0, Quat::IDENTITY);
        //     rotations.set_key_frame(32.0, Quat::from_rotation_x(PI));

        //     let rotations = animation.rotations.entry(1).or_default();
        //     rotations.set_key_frame(0.0, Quat::IDENTITY);
        //     rotations.set_key_frame(32.0, Quat::from_rotation_z(PI));

        //     animation
        // };

        Ok(Self {
            asset_loader,

            model: Some(model),
            animation,

            node_data_bind_group_layout,
            material_bind_group_layout,
            sampler,

            camera,
            camera_controller,
            gpu_camera,

            pipeline,
            depth_texture,

            models,
            model_to_load: None,

            animations,
            animation_to_load: None,

            gizmos_renderer,

            time: 0.0,
        })
    }

    fn load_model(&mut self, renderer: &Renderer, path: &Path) -> Result<Model, AssetError> {
        let smf = self.asset_loader.load_smf_direct(path)?;

        Model::from_smf(
            renderer,
            &self.asset_loader,
            &self.node_data_bind_group_layout,
            &self.material_bind_group_layout,
            &self.sampler,
            &smf,
        )
    }

    fn load_animation(
        &mut self,
        _renderer: &Renderer,
        path: &Path,
    ) -> Result<Animation, AssetError> {
        let bmf = self.asset_loader.load_bmf_direct(path)?;

        Animation::from_bmf(&bmf)
    }

    fn update_model_node_data(&self, renderer: &Renderer) {
        if let Some(ref model) = self.model {
            let node_data = Model::nodes_to_node_data(&model.nodes);

            renderer
                .queue
                .write_buffer(&model.nodes_buffer, 0, bytemuck::cast_slice(&node_data));
        }
    }

    fn update_animations(&mut self, _renderer: &Renderer) {
        let Some(ref mut model) = self.model else {
            return;
        };

        let Some(ref animation) = self.animation else {
            return;
        };

        for node in model.nodes.iter_mut() {
            let bone_id = node.bone_id as usize;
            let translation = if let Some(translations) = animation.translations.get(&bone_id) {
                translations.get(self.time)
            } else {
                Vec3::ZERO
            };
            let rotation = if let Some(rotations) = animation.rotations.get(&bone_id) {
                rotations.get(self.time)
            } else {
                Quat::IDENTITY
            };

            node.translation = translation;
            node.rotation = rotation;
        }
    }
}

impl Scene for ModelViewer {
    fn resize(&mut self, renderer: &Renderer) {
        // Resize the depth buffer.
        self.depth_texture =
            DepthBuffer::create_texture(&renderer.device, &renderer.surface_config);

        self.camera.aspect_ratio =
            renderer.surface_config.width as f32 / renderer.surface_config.height.max(1) as f32;

        self.camera_controller.dirty();
    }

    fn update(&mut self, renderer: &Renderer, delta_time: f32, input: &InputState) {
        if let Some(to_load) = self.model_to_load.take() {
            self.model = Some(self.load_model(renderer, &to_load).unwrap());
        }

        if let Some(ref to_load) = self.animation_to_load.take() {
            self.animation = Some(self.load_animation(renderer, to_load).unwrap());
        }

        self.camera_controller.on_input(input, delta_time);

        // Update the nodes from the animation data.
        self.update_animations(renderer);

        // Update the nodes on the gpu.
        self.update_model_node_data(renderer);
    }

    fn render(&mut self, frame: &mut Frame) {
        if self
            .camera_controller
            .update_camera_if_changed(&mut self.camera)
        {
            let matrices = self.camera.calculate_matrices();
            self.gpu_camera
                .upload_matrices(&frame.queue, &matrices, self.camera.position);
        }

        {
            let mut render_pass = frame
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("model_virewer_render_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &frame.surface,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.depth_texture,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

            let Some(ref model) = self.model else {
                return;
            };

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(1, &self.gpu_camera.bind_group, &[]);
            render_pass.set_bind_group(2, &model.nodes_bind_group, &[]);

            for mesh in model.meshes.iter() {
                render_pass.set_bind_group(0, &mesh.material_bind_group, &[]);
                render_pass.set_vertex_buffer(0, mesh.gpu_mesh.vertex_buffer.slice(..));
                render_pass.set_index_buffer(
                    mesh.gpu_mesh.index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                render_pass.draw_indexed(0..mesh.gpu_mesh.index_count, 0, 0..1);
            }
        }

        {
            const RED: Vec4 = Vec4::new(1.0, 0.0, 0.0, 1.0);
            const GREEN: Vec4 = Vec4::new(0.0, 1.0, 0.0, 1.0);
            const BLUE: Vec4 = Vec4::new(0.0, 0.0, 1.0, 1.0);

            let size = 10.0;
            let vertices = vec![
                GizmoVertex::new(Vec3::ZERO, RED),
                GizmoVertex::new(Vec3::X * size, RED),
                GizmoVertex::new(Vec3::ZERO, GREEN),
                GizmoVertex::new(Vec3::Y * size, GREEN),
                GizmoVertex::new(Vec3::ZERO, BLUE),
                GizmoVertex::new(Vec3::Z * size, BLUE),
            ];
            self.gizmos_renderer
                .render(frame, &self.gpu_camera.bind_group, &vertices);
        }

        if let Some(ref model) = self.model {
            // Draw the skeleton.
            let mut vertices = Vec::default();

            fn add_children(
                vertices: &mut Vec<GizmoVertex>,
                nodes: &[Node],
                node_index: usize,
                parent_position: Vec3,
            ) {
                const COLOR: Vec4 = Vec4::new(0.0, 1.0, 1.0, 1.0);

                let node = &nodes[node_index];

                let start = parent_position + node.translation;
                for (child_index, child) in nodes.iter().enumerate() {
                    if child.parent_index == node_index {
                        vertices.push(GizmoVertex::new(
                            start,
                            if node_index == 0 {
                                Vec4::new(1.0, 0.0, 0.0, 1.0)
                            } else {
                                COLOR
                            },
                        ));
                        vertices.push(GizmoVertex::new(start + child.translation, COLOR));

                        add_children(vertices, nodes, child_index, start);
                    }
                }
            }

            add_children(&mut vertices, &model.nodes, 0, Vec3::ZERO);

            self.gizmos_renderer
                .render(frame, &self.gpu_camera.bind_group, &vertices);
        }
    }

    fn debug_panel(&mut self, egui: &egui::Context, _renderer: &Renderer) {
        fn entries(dir_node: &DirNode, ui: &mut egui::Ui, to_load: &mut Option<PathBuf>) {
            for dir in dir_node.children.iter() {
                egui::CollapsingHeader::new(dir.0)
                    .default_open(false)
                    .show(ui, |ui| {
                        entries(dir.1, ui, to_load);
                    });
            }

            for file in dir_node.files.iter() {
                if ui.link(file.display().to_string()).clicked() {
                    *to_load = Some(file.clone());
                }
            }
        }

        egui::Window::new("Models").show(egui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                entries(&self.models, ui, &mut self.model_to_load);
            });
        });

        egui::Window::new("Animations").show(egui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                entries(&self.animations, ui, &mut self.animation_to_load);
            });
        });

        if self.animation.is_some() {
            egui::Window::new("Animation").show(egui, |ui| {
                if ui.button("Clear").clicked() {
                    self.animation = None;
                }
                ui.add(
                    egui::Slider::new(&mut self.time, 0.0..=32.0)
                        .drag_value_speed(0.1)
                        .step_by(0.1),
                );
            });
        }

        if let Some(ref mut model) = self.model {
            egui::Window::new("Model").show(egui, |ui| {
                fn do_node(ui: &mut egui::Ui, nodes: &mut [Node], parent_index: usize) {
                    // Gather up a list of child indices.
                    let children = nodes
                        .iter()
                        .enumerate()
                        .filter_map(|(child_index, node)| {
                            if node.parent_index == parent_index {
                                Some(child_index)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();

                    for child_index in children.iter() {
                        let node = &nodes[*child_index];
                        let node_label =
                            format!("{} {} ({})", child_index, node.name, node.parent_index);

                        ui.collapsing(node_label, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Translation");
                                ui.add(
                                    egui::DragValue::new(&mut nodes[*child_index].translation.x)
                                        .speed(0.01),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut nodes[*child_index].translation.y)
                                        .speed(0.01),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut nodes[*child_index].translation.z)
                                        .speed(0.01),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label("Rotation");
                                ui.add(
                                    egui::DragValue::new(&mut nodes[*child_index].rotation.x)
                                        .speed(0.01),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut nodes[*child_index].rotation.y)
                                        .speed(0.01),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut nodes[*child_index].rotation.z)
                                        .speed(0.01),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut nodes[*child_index].rotation.w)
                                        .speed(0.01),
                                );
                            });
                            do_node(ui, nodes, *child_index);
                        });
                    }
                }

                do_node(ui, &mut model.nodes, 0xFFFF_FFFF);
            });
        }
    }
}

struct Model {
    nodes: Vec<Node>,
    meshes: Vec<Mesh>,

    nodes_buffer: wgpu::Buffer,
    nodes_bind_group: wgpu::BindGroup,
}

impl Model {
    fn from_smf(
        renderer: &Renderer,
        asset_loader: &AssetLoader,
        node_data_bind_group_layout: &wgpu::BindGroupLayout,
        material_bind_group_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        smf: &smf::Model,
    ) -> Result<Self, AssetError> {
        let mut nodes = Vec::default();
        let mut meshes = Vec::default();
        let mut node_names = HashMap::with_capacity(smf.nodes.len());

        for (node_index, smf_node) in smf.nodes.iter().enumerate() {
            node_names.insert(smf_node.name.clone(), node_index);

            let bone_id = smf_node.tree_id;
            let parent_index = node_names
                .get(&smf_node.parent_name)
                .cloned()
                .unwrap_or(0xFFFF_FFFF);

            let node_index = nodes.len();
            nodes.reserve(smf_node.meshes.len());
            for smf_mesh in smf_node.meshes.iter() {
                meshes.push(Mesh::from_smf_mesh(
                    renderer,
                    asset_loader,
                    material_bind_group_layout,
                    sampler,
                    smf_mesh,
                    node_index,
                )?);
            }

            println!("bone_id: {bone_id}, rotation: {}", smf_node.rotation);

            nodes.push(Node {
                parent_index,
                bone_id,

                translation: smf_node.position,
                rotation: smf_node.rotation,
                name: smf_node.name.clone(),
            });
        }

        // println!("nodes: {:#?}", nodes);

        let nodes_buffer = Self::create_nodes_buffer(renderer, &nodes);
        let nodes_bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("nodes_bind_group"),
                layout: node_data_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: nodes_buffer.as_entire_binding(),
                }],
            });

        Ok(Self {
            nodes,
            meshes,
            nodes_buffer,
            nodes_bind_group,
        })
    }

    fn nodes_to_node_data(nodes: &[Node]) -> Vec<NodeData> {
        let mut new_nodes: Vec<NodeData> = Vec::with_capacity(nodes.len());

        for node in nodes.iter() {
            let parent_transform = if node.parent_index == 0xFFFF_FFFF {
                Mat4::IDENTITY
            } else {
                new_nodes[node.parent_index].transform
            };

            let transform =
                parent_transform * Mat4::from_rotation_translation(node.rotation, node.translation);

            new_nodes.push(NodeData {
                transform,
                parent: node.parent_index as u32,
                _pad: [0; 3],
            });
        }

        new_nodes
    }

    fn create_nodes_buffer(renderer: &Renderer, nodes: &[Node]) -> wgpu::Buffer {
        let node_data = Self::nodes_to_node_data(nodes);

        renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("nodes_buffer"),
                contents: bytemuck::cast_slice(&node_data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            })
    }
}

#[derive(Clone, Debug)]
struct Node {
    parent_index: usize,
    bone_id: u32,

    translation: Vec3,
    rotation: Quat,
    name: String,
}

struct Mesh {
    node_index: usize,
    gpu_mesh: GpuIndexedMesh,
    material_bind_group: wgpu::BindGroup,
}

impl Mesh {
    fn from_smf_mesh(
        renderer: &Renderer,
        asset_loader: &AssetLoader,
        material_bind_group_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        smf_mesh: &smf::Mesh,
        node_index: usize,
    ) -> Result<Self, AssetError> {
        let vertices = smf_mesh
            .vertices
            .iter()
            .map(|v| Vertex {
                position: v.position,
                normal: v.normal,
                tex_coord: v.tex_coord,
                node_index: node_index as u32,
            })
            .collect();

        let indices = smf_mesh.faces.iter().flat_map(|f| f.indices).collect();

        let gpu_mesh = IndexedMesh { vertices, indices }.to_gpu(renderer);

        let material_bind_group = {
            let path = PathBuf::from("textures")
                .join("shared")
                .join(&smf_mesh.texture_name);
            let image = match asset_loader.load_bmp_direct(&path) {
                Ok(image) => image,
                Err(_) => {
                    tracing::warn!(
                        "Could not load {}. Loading error.bmp instead.",
                        path.display()
                    );
                    asset_loader.load_bmp_direct(
                        &PathBuf::from("textures").join("object").join("error.bmp"),
                    )?
                }
            };
            let texture = renderer
                .create_texture_view(&format!("texture ({})", smf_mesh.texture_name), &image.data);

            renderer
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(&format!("material_bind_group ({})", smf_mesh.texture_name)),
                    layout: material_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&texture),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(sampler),
                        },
                    ],
                })
        };

        Ok(Self {
            node_index,
            gpu_mesh,
            material_bind_group,
        })
    }
}

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
struct Vertex {
    position: Vec3,
    normal: Vec3,
    tex_coord: Vec2,
    node_index: u32,
}

impl BufferLayout for Vertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        const VERTEX_ATTR_ARRAY: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array!(
            0 => Float32x3, // position
            1 => Float32x3, // normal
            2 => Float32x2, // tex_coord
            3 => Uint32,    // node_index
        );

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: VERTEX_ATTR_ARRAY,
        }
    }
}

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
struct NodeData {
    transform: Mat4,
    parent: u32,
    _pad: [u32; 3],
}

#[derive(Default)]
struct DirNode {
    files: Vec<PathBuf>,
    children: BTreeMap<String, DirNode>,
}

impl DirNode {
    fn insert(&mut self, path: PathBuf) {
        let mut current = self;
        for component in path.iter() {
            let name = component.to_string_lossy().into_owned();
            if name.contains('.') {
                // Assuming files have extensions
                current.files.push(path.clone());
            } else {
                current = current.children.entry(name).or_default();
            }
        }
    }
}

fn do_node(ui: &mut egui::Ui, nodes: &mut [Node], node_index: usize) {
    let child_indices = nodes
        .iter()
        .enumerate()
        .filter_map(|(child_index, node)| (node.parent_index == node_index).then_some(child_index))
        .collect::<Vec<_>>();

    let node = &mut nodes[node_index];

    ui.horizontal(|ui| {
        ui.label("Translation");
        ui.add(egui::DragValue::new(&mut node.translation.x));
        ui.add(egui::DragValue::new(&mut node.translation.y));
        ui.add(egui::DragValue::new(&mut node.translation.z));
    });

    egui::CollapsingHeader::new(&node.name).show(ui, |ui| {
        for child_index in child_indices.iter() {
            do_node(ui, nodes, *child_index);
        }
    });
}

#[derive(Default)]
struct Animation {
    translations: HashMap<usize, Track<Vec3>>,
    rotations: HashMap<usize, Track<Quat>>,
}

impl Animation {
    fn from_bmf(bmf: &bmf::Motion) -> Result<Self, AssetError> {
        let mut animation = Animation::default();

        for key_frame in bmf.key_frames.iter() {
            for bone in key_frame.bones.iter() {
                let time = bone.time as f32;
                let bone_id = bone.bone_index as usize;

                if let Some(position) = bone.position {
                    animation
                        .translations
                        .entry(bone_id)
                        .or_default()
                        .set_key_frame(time, position);
                }

                if let Some(rotation) = bone.rotation {
                    animation
                        .rotations
                        .entry(bone_id)
                        .or_default()
                        .set_key_frame(
                            time,
                            Quat::from_xyzw(rotation.x, rotation.y, -rotation.z, rotation.w),
                        );
                }
            }
        }

        Ok(animation)
    }

    fn translation_at(&self, bone_index: usize, time: f32) -> Option<Vec3> {
        self.translations
            .get(&bone_index)
            .map(|track| track.get(time))
    }

    fn rotation_at(&self, bone_index: usize, time: f32) -> Option<Quat> {
        self.rotations.get(&bone_index).map(|track| track.get(time))
    }
}
