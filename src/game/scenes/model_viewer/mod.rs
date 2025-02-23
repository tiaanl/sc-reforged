use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use shadow_company_tools::smf;
use wgpu::util::DeviceExt;

use crate::engine::prelude::*;
use crate::game::asset_loader::{AssetError, AssetLoader};
use crate::game::camera;

pub struct ModelViewer {
    asset_loader: Arc<AssetLoader>,

    model: Option<Model>,

    node_data_bind_group_layout: wgpu::BindGroupLayout,
    material_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,

    camera: camera::Camera,
    camera_controller: camera::ArcBallCameraController,
    gpu_camera: camera::GpuCamera,

    depth_texture: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,

    dirs: DirNode,
    to_load: Option<PathBuf>,
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
                    buffers: Vertex::vertex_buffers(),
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

        let mut dirs = DirNode::default();
        for path in asset_loader
            .enum_dir(PathBuf::from("models"))?
            .iter()
            .filter(|p| p.extension().map(|e| e == "smf").unwrap_or(false))
        {
            dirs.insert(path.clone());
        }

        Ok(Self {
            asset_loader,

            model: None,

            node_data_bind_group_layout,
            material_bind_group_layout,
            sampler,

            camera,
            camera_controller,
            gpu_camera,

            pipeline,
            depth_texture,

            dirs,
            to_load: None,
        })
    }

    fn load_model(
        &mut self,
        renderer: &Renderer,
        path: impl AsRef<Path>,
    ) -> Result<Model, AssetError> {
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
        if let Some(ref to_load) = self.to_load.take() {
            self.model = Some(self.load_model(renderer, to_load).unwrap());
        }

        self.camera_controller.on_input(input, delta_time);
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

        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("model_virewer_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.surface,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
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

    fn debug_panel(&mut self, egui: &egui::Context, _renderer: &Renderer) {
        fn dir_ui(dir_node: &DirNode, ui: &mut egui::Ui, to_load: &mut Option<PathBuf>) {
            for dir in dir_node.children.iter() {
                egui::CollapsingHeader::new(dir.0)
                    .default_open(false)
                    .show(ui, |ui| {
                        dir_ui(dir.1, ui, to_load);
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
                dir_ui(&self.dirs, ui, &mut self.to_load);
            });
        });

        egui::Window::new("Model").show(egui, |ui| {
            ui.label("test");
        });
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

            let local_transform = Mat4::from_translation(smf_node.position);
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

            nodes.push(Node {
                parent_index,
                bone_id,
                local_transform,
            });
        }

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

    fn create_nodes_buffer(renderer: &Renderer, nodes: &[Node]) -> wgpu::Buffer {
        let node_data = nodes
            .iter()
            .map(|node| NodeData {
                mat_model: node.local_transform.to_cols_array_2d(),
                parent: node.parent_index as u32,
                _pad: [0; 3],
            })
            .collect::<Vec<_>>();

        renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("nodes_buffer"),
                contents: bytemuck::cast_slice(&node_data),
                usage: wgpu::BufferUsages::STORAGE,
            })
    }
}

struct Node {
    parent_index: usize,
    bone_id: u32,
    local_transform: Mat4,
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
            let image = asset_loader.load_bmp_direct(
                PathBuf::from("textures")
                    .join("shared")
                    .join(&smf_mesh.texture_name),
            )?;
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
    fn vertex_buffers() -> &'static [wgpu::VertexBufferLayout<'static>] {
        use wgpu::vertex_attr_array;

        const VERTEX_ATTR_ARRAY: &[wgpu::VertexAttribute] = &vertex_attr_array!(
            0 => Float32x3, // position
            1 => Float32x3, // normal
            2 => Float32x2, // tex_coord
            3 => Uint32,    // node_index
        );

        &[wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: VERTEX_ATTR_ARRAY,
        }]
    }
}

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
struct NodeData {
    mat_model: [[f32; 4]; 4],
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
