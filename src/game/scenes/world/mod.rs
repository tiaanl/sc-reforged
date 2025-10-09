use std::path::PathBuf;

use terrain::Terrain;
use wgpu::util::DeviceExt;

use crate::{
    engine::{
        gizmos::{GizmoVertex, GizmosRenderer},
        prelude::*,
    },
    game::{
        camera::{self, Controller},
        compositor::Compositor,
        config::{CampaignDef, ObjectType},
        data_dir::data_dir,
        geometry_buffers::{GeometryBuffers, GeometryData, RenderTarget},
        image::images,
        scenes::world::{
            actions::PlayerAction,
            game_mode::GameMode,
            new_terrain::NewTerrain,
            overlay_renderer::OverlayRenderer,
            quad_tree::QuadTree,
            render_world::RenderWorld,
            sim_world::{ComputedCamera, SimWorld},
            systems::RenderStore,
        },
        shadows::ShadowCascades,
        sky_renderer::SkyRenderer,
    },
};

pub mod actions;
mod game_mode;
pub mod new_height_map;
mod new_terrain;
mod object;
mod objects;
mod overlay_renderer;
mod quad_tree;
mod render_world;
mod sim_world;
mod strata;
mod systems;
pub mod terrain;

#[derive(Clone, Copy, Default, bytemuck::NoUninit)]
#[repr(C)]
struct GpuEnvironment {
    pub sun_dir: [f32; 4],
    pub sun_color: [f32; 4],

    pub fog_color: [f32; 4],
    pub fog_params: [f32; 4],
}

/// Wrap all the data for controlling the camera.
struct Camera<C: camera::Controller> {
    controller: C,
    camera: camera::Camera,
    gpu_camera: camera::GpuCamera,
}

/// The [Scene] that renders the ingame world view.
pub struct WorldScene {
    sim_world: SimWorld,
    render_worlds: [RenderWorld; Self::RENDER_FRAME_COUNT],
    render_store: RenderStore,

    // Systems
    systems: systems::Systems,

    game_mode: GameMode,

    view_debug_camera: bool,
    control_debug_camera: bool,

    main_camera: Camera<camera::GameCameraController>,
    debug_camera: Camera<camera::FreeCameraController>,

    terrain: Terrain,
    objects: objects::Objects,
    quad_tree: QuadTree,

    // Render
    depth_buffer: wgpu::TextureView,

    shadow_cascades: ShadowCascades,
    shadow_cascades_lambda: f32,
    geometry_buffers: GeometryBuffers,
    compositor: Compositor,
    shadow_render_target: RenderTarget,

    sky_renderer: SkyRenderer,

    overlay_renderer: OverlayRenderer,

    environment: GpuEnvironment,
    ambient_color: Vec3,

    environment_buffer: wgpu::Buffer,
    environment_bind_group: wgpu::BindGroup,

    last_mouse_position: Option<UVec2>,
    geometry_data: Option<GeometryData>,

    last_frame_time: std::time::Instant,
    fps_history: Vec<f32>,
    fps_history_cursor: usize,

    gizmos_vertices: Vec<GizmoVertex>,
    gizmos_renderer: GizmosRenderer,

    render_overlay: bool,

    pos_and_normal: Option<(Vec3, Vec3)>,
}

impl WorldScene {
    const RENDER_FRAME_COUNT: usize = 3;
    const SHADOW_MAP_RESOLUTION: u32 = 2048;

    pub fn new(campaign_def: CampaignDef) -> Result<Self, AssetError> {
        tracing::info!("Loading campaign \"{}\"...", campaign_def.title);

        let campaign = data_dir().load_campaign(&campaign_def.base_name)?;

        let main_camera = {
            let camera_from = campaign.view_initial.from.extend(2500.0);
            let camera_to = campaign.view_initial.to.extend(0.0);

            let mut controller = camera::GameCameraController::new(1000.0, 0.2);
            controller.move_to_direct(camera_from);
            controller.look_at_direct(camera_to);
            let camera = camera::Camera::new(
                camera_from,
                Quat::IDENTITY,
                45.0_f32.to_radians(),
                1.0,
                10.0,
                13_300.0,
            );
            let gpu_camera = camera::GpuCamera::new(&renderer().device);

            Camera {
                controller,
                camera,
                gpu_camera,
            }
        };

        let debug_camera = {
            let controller = camera::FreeCameraController::new(1000.0, 0.2);
            let camera = camera::Camera::new(
                Vec3::new(0.0, 0.0, 10_000.0),
                Quat::IDENTITY,
                45.0_f32.to_radians(),
                1.0,
                10.0,
                150_000.0,
            );
            let gpu_camera = camera::GpuCamera::new(&renderer().device);

            Camera {
                controller,
                camera,
                gpu_camera,
            }
        };

        let time_of_day = 12.0;
        let day_night_cycle = {
            use sim_world::DayNightCycle;

            let mut result = DayNightCycle::default();

            campaign.time_of_day.iter().enumerate().for_each(|(i, t)| {
                let index = i as u32;

                result.sun_dir.insert(index, t.sun_dir);
                result.sun_color.insert(index, t.sun_color);

                result.fog_distance.insert(index, t.fog_distance);
                result.fog_near_fraction.insert(index, t.fog_near_fraction);
                result.fog_color.insert(index, t.fog_color);
            });

            result
        };

        let environment = GpuEnvironment::default();
        let ambient_color = Vec3::splat(0.3);

        let environment_buffer =
            renderer()
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("environment_buffer"),
                    contents: bytemuck::cast_slice(&[environment]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let environment_bind_group_layout =
            renderer()
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("environment_bind_group_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX
                            | wgpu::ShaderStages::FRAGMENT
                            | wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let environment_bind_group =
            renderer()
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("environment_bind_group_layout"),
                    layout: &environment_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(
                            environment_buffer.as_entire_buffer_binding(),
                        ),
                    }],
                });

        let shadow_cascades = ShadowCascades::new(&renderer().device, Self::SHADOW_MAP_RESOLUTION);

        let geometry_buffers = GeometryBuffers::new(&renderer().device, renderer().surface.size());

        // Start with a 4K shadow buffer.
        let shadow_render_target = RenderTarget::new(
            &renderer().device,
            "shadow",
            UVec2::new(Self::SHADOW_MAP_RESOLUTION, Self::SHADOW_MAP_RESOLUTION),
            wgpu::TextureFormat::Depth32Float,
        );

        let compositor = Compositor::new(
            &geometry_buffers.bind_group_layout,
            &main_camera.gpu_camera.bind_group_layout,
            &environment_bind_group_layout,
        );

        let terrain = Terrain::new(
            &campaign_def,
            &main_camera.gpu_camera.bind_group_layout,
            &environment_bind_group_layout,
            &shadow_cascades,
        )?;

        let mut objects = objects::Objects::new(
            &main_camera.gpu_camera.bind_group_layout,
            &environment_bind_group_layout,
            &shadow_cascades,
        )?;

        let quad_tree = QuadTree::default();

        if let Some(ref mtf_name) = campaign.mtf_name {
            let mtf = data_dir().load_mtf(mtf_name)?;

            for object in mtf.objects.iter() {
                let object_type = ObjectType::from_string(&object.typ)
                    .unwrap_or_else(|| panic!("missing object type: {}", object.typ));

                if let Err(err) = objects.spawn(
                    // Rotate objects to the left.
                    Transform::from_translation(object.position)
                        .with_euler_rotation(object.rotation * Vec3::new(1.0, 1.0, -1.0)),
                    object_type,
                    &object.name,
                    &object.title,
                ) {
                    tracing::error!("Could not load model: {}", err);
                }
            }
        }

        let sky_renderer = {
            let device = &renderer().device;
            let queue = &renderer().queue;

            let mut sky_renderer =
                SkyRenderer::new(device, &main_camera.gpu_camera.bind_group_layout);

            for sky_texture in campaign.sky_textures.iter() {
                // Kind of hacky, but OK.
                if sky_texture.name.eq_ignore_ascii_case("unused.bmp") {
                    continue;
                }

                let image = images().load_image(
                    PathBuf::from("textures")
                        .join("object")
                        .join(&sky_texture.name),
                )?;
                sky_renderer.set_sky_texture(device, queue, sky_texture.index, image);
            }

            sky_renderer
        };

        let overlay_renderer = OverlayRenderer::new(
            &main_camera.gpu_camera.bind_group_layout,
            &shadow_cascades,
            &geometry_buffers,
        );

        let gizmos_renderer = GizmosRenderer::new(&main_camera.gpu_camera.bind_group_layout);

        let fps_history = vec![0.0; 100];
        let fps_history_cursor = 0;

        let sim_world = {
            let terrain_mapping = data_dir().load_terrain_mapping(&campaign_def.base_name)?;

            let height_map = {
                {
                    let path =
                        PathBuf::from("maps").join(format!("{}.pcx", &campaign_def.base_name));
                    tracing::info!("Loading terrain height map: {}", path.display());
                    data_dir().load_new_height_map(
                        path,
                        terrain_mapping.altitude_map_height_base,
                        terrain_mapping.nominal_edge_size,
                    )?
                }
            };

            let terrain = {
                let terrain_texture =
                    data_dir().load_terrain_texture(&terrain_mapping.texture_map_base_name)?;

                NewTerrain::new(height_map, terrain_texture)
            };

            let quad_tree = QuadTree::from_new_terrain(&terrain);

            SimWorld {
                camera: camera::Camera::new(
                    Vec3::ZERO,
                    Quat::IDENTITY,
                    45.0_f32.to_radians(),
                    1.0,
                    10.0,
                    13_300.0,
                ),
                computed_camera: ComputedCamera::default(),
                time_of_day,
                day_night_cycle,

                quad_tree,

                terrain,
                visible_chunks: Vec::default(),

                gizmo_vertices: Vec::with_capacity(1024),
            }
        };

        let depth_buffer = Self::create_depth_buffer(&renderer().device, renderer().surface.size());

        let render_store = RenderStore::new(renderer());

        let render_worlds = [
            RenderWorld::new(0, renderer(), &render_store),
            RenderWorld::new(1, renderer(), &render_store),
            RenderWorld::new(2, renderer(), &render_store),
        ];

        let systems = systems::Systems::new(renderer(), &render_store, &sim_world, &campaign);

        Ok(Self {
            sim_world,
            render_worlds,
            render_store,

            systems,

            game_mode: GameMode::Editor,

            view_debug_camera: false,
            control_debug_camera: false,

            main_camera,
            debug_camera,

            terrain,
            objects,
            quad_tree,

            depth_buffer,
            shadow_cascades,
            shadow_cascades_lambda: 0.5,
            geometry_buffers,
            shadow_render_target,
            compositor,

            sky_renderer,

            overlay_renderer,

            environment,
            ambient_color,

            environment_buffer,
            environment_bind_group,

            last_mouse_position: None,
            geometry_data: None,

            last_frame_time: std::time::Instant::now(),
            fps_history,
            fps_history_cursor,

            gizmos_vertices: Vec::with_capacity(128),
            gizmos_renderer,

            render_overlay: false,

            pos_and_normal: None,
        })
    }

    pub fn in_editor(&self) -> bool {
        matches!(self.game_mode, GameMode::Editor)
    }

    fn calculate_environment(&self, time_of_day: f32) -> GpuEnvironment {
        let day_night_cycle = &self.sim_world.day_night_cycle;

        let sun_dir = day_night_cycle.sun_dir.sample_sub_frame(time_of_day, true);
        let sun_color = day_night_cycle
            .sun_color
            .sample_sub_frame(time_of_day, true);

        let fog_far = day_night_cycle
            .fog_distance
            .sample_sub_frame(time_of_day, true);
        let fog_near = fog_far
            * day_night_cycle
                .fog_near_fraction
                .sample_sub_frame(time_of_day, true);
        let fog_color = day_night_cycle
            .fog_color
            .sample_sub_frame(time_of_day, true);

        GpuEnvironment {
            sun_dir: sun_dir.extend(self.ambient_color.x).to_array(),
            sun_color: sun_color.extend(self.ambient_color.y).to_array(),
            fog_color: fog_color.extend(self.ambient_color.z).to_array(),
            fog_params: [fog_near, fog_far, 0.0, 0.0],
        }
    }

    fn create_depth_buffer(device: &wgpu::Device, size: UVec2) -> wgpu::TextureView {
        let size = wgpu::Extent3d {
            width: size.x.max(1),
            height: size.y.max(1),
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth_buffer"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }
}

impl Scene for WorldScene {
    fn resize(&mut self) {
        let size = renderer().surface.size();

        self.depth_buffer = Self::create_depth_buffer(&renderer().device, size);

        // Replace the buffers with new ones.
        self.geometry_buffers = GeometryBuffers::new(&renderer().device, size);

        let [width, height] = size.to_array().map(|f| f as f32);
        let aspect = width / height.max(1.0);
        self.main_camera.camera.aspect_ratio = aspect;
        self.debug_camera.camera.aspect_ratio = aspect;

        self.sim_world.camera.aspect_ratio = aspect;
    }

    fn event(&mut self, event: SceneEvent) {
        match event {
            SceneEvent::MouseLeft => self.last_mouse_position = None,
            SceneEvent::MouseMove { position } => {
                self.last_mouse_position = Some(position);
            }
            _ => {}
        }
    }

    fn update(&mut self, delta_time: f32, input: &InputState) {
        // Run systems
        {
            let time = systems::Time { delta_time };
            self.systems.input(&mut self.sim_world, &time, input);
            self.systems.update(&mut self.sim_world, &time);
        }

        self.pos_and_normal = self.geometry_data.as_ref().map(|data| {
            let world_xy = data.position.truncate();
            self.terrain.height_map.world_position_and_normal(world_xy)
        });

        if input.key_just_pressed(KeyCode::Backquote) {
            self.game_mode = if self.in_editor() {
                GameMode::Game
            } else {
                GameMode::Editor
            }
        }

        if input.mouse_just_pressed(MouseButton::Left) {
            if let Some(ref data) = self.geometry_data {
                // TODO: This needs a better place.
                const TERRAIN_ENTITY_ID: u32 = 1 << 16;

                // Figure out the type of object we clicked on:
                let player_action = if data.id >= TERRAIN_ENTITY_ID {
                    PlayerAction::TerrainClicked {
                        _position: data.position,
                    }
                } else {
                    PlayerAction::ObjectClicked {
                        _position: data.position,
                        id: data.id,
                    }
                };

                self.objects.handle_player_action(&player_action);
            }
        } else if input.mouse_just_pressed(MouseButton::Right) {
            self.objects
                .handle_player_action(&PlayerAction::ClearSelection);
        }

        // Advance time of day.
        // self.time_of_day = (self.time_of_day + delta_time * 0.01).rem_euclid(24.0);
        self.environment = self.calculate_environment(self.sim_world.time_of_day);

        // Set the camera far plane to max distance of the fog.
        self.main_camera.camera.far = self.environment.fog_params[1];
        if self.control_debug_camera {
            self.debug_camera.controller.update(delta_time, input);
        } else {
            self.main_camera.controller.update(delta_time, input);
        }

        self.main_camera
            .controller
            .update_camera(&mut self.main_camera.camera);
        self.debug_camera
            .controller
            .update_camera(&mut self.debug_camera.camera);

        self.objects.update(delta_time, &self.terrain.height_map);
    }

    fn render(&mut self, frame: &mut Frame) {
        let render_world = &mut self.render_worlds[frame.frame_index % Self::RENDER_FRAME_COUNT];

        // Systems
        {
            self.systems.extract(&mut self.sim_world, render_world);
            self.systems
                .prepare(render_world, renderer(), &mut self.render_store);
            self.systems
                .queue(render_world, frame, &self.depth_buffer, &self.render_store);
        }

        /*
        let main_view_projection = {
            let camera = &self.main_camera.camera;

            let view_projection = camera.calculate_view_projection();
            let forward = view_projection.mat.project_point3(Vec3::Y).normalize();
            let position = camera.position;
            let fov = camera.fov;
            let aspect_ratio = camera.aspect_ratio;

            self.main_camera.gpu_camera.upload(
                &view_projection,
                position,
                forward,
                fov,
                aspect_ratio,
            );

            view_projection
        };

        let main_camera_frustum = main_view_projection.frustum();

        let _debug_view_projection = {
            let camera = &self.debug_camera.camera;

            let view_projection = camera.calculate_view_projection();
            let position = camera.position;
            let forward = view_projection.mat.project_point3(Vec3::Y).normalize();
            let fov = camera.fov;
            let aspect_ratio = camera.aspect_ratio;

            self.debug_camera.gpu_camera.upload(
                &view_projection,
                position,
                forward,
                fov,
                aspect_ratio,
            );

            view_projection
        };

        let sun_dir = Vec4::from(self.environment.sun_dir).truncate();
        self.shadow_cascades.update_from_camera(
            &self.main_camera.camera,
            sun_dir,
            self.shadow_cascades_lambda,
        );

        renderer().queue.write_buffer(
            &self.environment_buffer,
            0,
            bytemuck::cast_slice(&[self.environment]),
        );

        // Clear the buffers.
        {
            const INVALID_ID: u32 = 0xFFFF_FFFF;
            let positions_clear_color = wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: f32::from_le_bytes(INVALID_ID.to_le_bytes()) as f64,
            };
            let fog_clear_color = wgpu::Color {
                r: self.environment.fog_color[0] as f64,
                g: self.environment.fog_color[1] as f64,
                b: self.environment.fog_color[2] as f64,
                a: 1.0,
            };

            let oit_accumulation_clear = wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            };
            let oit_revealage_clear = wgpu::Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            };

            drop(
                frame
                    .encoder
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("world_clear_render_pass"),
                        color_attachments: &[
                            // Clear the color buffer with the fog color.
                            Some(wgpu::RenderPassColorAttachment {
                                view: &self.geometry_buffers.color.view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(fog_clear_color),
                                    store: wgpu::StoreOp::Store,
                                },
                            }),
                            Some(wgpu::RenderPassColorAttachment {
                                view: &self.geometry_buffers.oit_accumulation.view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(oit_accumulation_clear),
                                    store: wgpu::StoreOp::Store,
                                },
                            }),
                            Some(wgpu::RenderPassColorAttachment {
                                view: &self.geometry_buffers.oit_revealage.view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(oit_revealage_clear),
                                    store: wgpu::StoreOp::Store,
                                },
                            }),
                            // Set positions to 0 and ID's to invalid.
                            Some(wgpu::RenderPassColorAttachment {
                                view: &self.geometry_buffers.position_id.view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(positions_clear_color),
                                    store: wgpu::StoreOp::Store,
                                },
                            }),
                        ],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &self.geometry_buffers.depth.view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    }),
            );

            drop(
                frame
                    .encoder
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("light_clear_render_pass"),
                        color_attachments: &[],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &self.shadow_render_target.view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    }),
            );

            self.shadow_cascades.clear_buffers(&mut frame.encoder);
        }

        // --- Shadow pass ---

        let _z = tracy_client::span!("render shadow map");

        if true {
            self.objects
                .render_shadow_casters(frame, &self.shadow_cascades);
        }

        // --- Color pass ---

        let view_camera_bind_group = if self.in_editor() && self.view_debug_camera {
            &self.debug_camera.gpu_camera.bind_group
        } else {
            &self.main_camera.gpu_camera.bind_group
        };

        // --- Sky box ---
        self.sky_renderer.render(
            frame,
            &self.geometry_buffers,
            &self.main_camera.gpu_camera.bind_group,
        );

        if true {
            // Render Opaque geometry first.
            self.terrain.render(
                frame,
                self.in_editor(),
                &self.geometry_buffers,
                &self.shadow_cascades,
                view_camera_bind_group,
                &self.environment_bind_group,
                &self.main_camera.gpu_camera.bind_group, // Always the main camera.
            );

            self.objects.render_objects(
                frame,
                &main_camera_frustum,
                &self.geometry_buffers,
                view_camera_bind_group,
                &self.environment_bind_group,
                &self.shadow_cascades,
            );

            // Now render alpha geoometry.
            self.terrain.render_water(
                frame,
                &self.geometry_buffers,
                view_camera_bind_group,
                &self.environment_bind_group,
            );
        }

        if self.in_editor() && self.render_overlay {
            self.overlay_renderer.render(
                frame,
                &self.main_camera.gpu_camera.bind_group,
                &self.shadow_cascades,
                &self.geometry_buffers,
            );
        } else {
            self.compositor.render(
                frame,
                &self.geometry_buffers,
                &self.main_camera.gpu_camera.bind_group,
                &self.environment_bind_group,
            );
        }

        if self.in_editor() {
            let _z = tracy_client::span!("render gizmos");

            self.gizmos_vertices.clear();

            // Render any kind of debug overlays.
            self.terrain.render_gizmos(&mut self.gizmos_vertices);
            self.objects.render_gizmos(&mut self.gizmos_vertices);

            if let Some((pos, normal)) = self.pos_and_normal {
                let color = Vec4::new(1.0, 0.0, 0.0, 1.0);
                self.gizmos_vertices.push(GizmoVertex::new(pos, color));
                self.gizmos_vertices
                    .push(GizmoVertex::new(pos + normal * 100.0, color));
            }

            // Render the main camera frustum when we're looking through the debug camera.
            // if self.view_debug_camera {
            //     self.gizmos_vertices
            //         .extend(GizmosRenderer::create_view_projection(
            //             &main_view_projection,
            //             Vec4::new(1.0, 0.0, 1.0, 1.0),
            //         ));
            // }

            // if false {
            //     if let Some(ref data) = self.geometry_data {
            //         // Translation matrix
            //         let translation = Mat4::from_translation(data.position);

            //         let vertices = GizmosRenderer::create_axis(translation, 100.0);
            //         self.gizmos_renderer
            //             .render(frame, view_camera_bind_group, &vertices);
            //     }
            // }

            // Render sky box.
            {
                let blue = Vec4::new(0.0, 0.0, 1.0, 1.0);

                const RADIUS_MID: f32 = 17_100.0;
                const RADIUS_FAR: f32 = 25_650.0;

                // Use the horizontal fov to calculate the edges.
                let half_angle = ((self.main_camera.camera.fov * 0.5).tan()
                    * self.main_camera.camera.aspect_ratio)
                    .atan();
                // Add some overscan.
                let half_angle = half_angle * 1.08;

                // Rotate forward_xy around +Z by ±`half_angle` to get the two edge directions.
                let rot_neg = Quat::from_axis_angle(Vec3::Z, -half_angle);
                let rot_pos = Quat::from_axis_angle(Vec3::Z, half_angle);

                // Get the world forward from the camera.
                let forward_world = self.main_camera.camera.rotation.mul_vec3(Vec3::Y);
                let forward_xy = {
                    let v = Vec2::new(forward_world.x, forward_world.y);
                    let len = v.length();
                    if len > 1e-6 { v / len } else { Vec2::Y }
                };

                let edge0 = (rot_neg.mul_vec3(forward_xy.extend(0.0))).truncate(); // Left edge.
                let edge1 = (rot_pos.mul_vec3(forward_xy.extend(0.0))).truncate(); // Right edge.

                // Build the ring points.
                let cam_pos = self.main_camera.camera.position.truncate();
                let mid0 = cam_pos + edge0 * RADIUS_MID;
                let mid1 = cam_pos + edge1 * RADIUS_MID;
                let far0 = cam_pos + edge0 * RADIUS_FAR;
                let far1 = cam_pos + edge1 * RADIUS_FAR;

                const Z_APEX_HIGH: f32 = 7_500.0;
                const Z_RING_HIGH: f32 = 2_500.0;
                const Z_FAR_HIGH: f32 = 0.0;
                const Z_OFFSET_LOW: f32 = -100.0;

                // Top apex.
                let apex_high = Vec3::new(cam_pos.x, cam_pos.y, Z_APEX_HIGH);
                let mid0_high = Vec3::new(mid0.x, mid0.y, Z_RING_HIGH);
                let mid1_high = Vec3::new(mid1.x, mid1.y, Z_RING_HIGH);
                let far0_high = Vec3::new(far0.x, far0.y, Z_FAR_HIGH);
                let far1_high = Vec3::new(far1.x, far1.y, Z_FAR_HIGH);

                let high = [apex_high, mid0_high, mid1_high, far0_high, far1_high];

                let apex_low = apex_high + Vec3::Z * Z_OFFSET_LOW;
                let mid0_low = mid0_high + Vec3::Z * Z_OFFSET_LOW;
                let mid1_low = mid1_high + Vec3::Z * Z_OFFSET_LOW;
                let far0_low = far0_high + Vec3::Z * Z_OFFSET_LOW;
                let far1_low = far1_high + Vec3::Z * Z_OFFSET_LOW;
                let low = [apex_low, mid0_low, mid1_low, far0_low, far1_low];

                let indices = [0usize, 2, 1, 1, 2, 4, 1, 4, 3];

                for a in indices.windows(3) {
                    self.gizmos_vertices.extend(vec![
                        GizmoVertex::new(high[a[0]], blue),
                        GizmoVertex::new(high[a[1]], blue),
                        GizmoVertex::new(high[a[1]], blue),
                        GizmoVertex::new(high[a[2]], blue),
                        GizmoVertex::new(high[a[2]], blue),
                        GizmoVertex::new(high[a[0]], blue),
                    ]);
                    self.gizmos_vertices.extend(vec![
                        GizmoVertex::new(low[a[0]], blue),
                        GizmoVertex::new(low[a[1]], blue),
                        GizmoVertex::new(low[a[1]], blue),
                        GizmoVertex::new(low[a[2]], blue),
                        GizmoVertex::new(low[a[2]], blue),
                        GizmoVertex::new(low[a[0]], blue),
                    ]);
                }
            }

            // self.quad_tree.render_gizmos(&mut self.gizmos_vertices);
            // self.quad_tree
            //     .render_gizmos_in_frustum(&main_camera_frustum, &mut self.gizmos_vertices);

            // self.gizmos_renderer
            //     .render(frame, view_camera_bind_group, &self.gizmos_vertices);
        }

        let now = std::time::Instant::now();
        let render_time = now - self.last_frame_time;
        self.last_frame_time = now;

        self.fps_history[self.fps_history_cursor] = render_time.as_secs_f32();
        self.fps_history_cursor = (self.fps_history_cursor + 1) % self.fps_history.len();
        */
    }

    fn post_render(&mut self) {
        self.geometry_data = self.last_mouse_position.map(|position| {
            self.geometry_buffers
                .fetch_data(&renderer().device, &renderer().queue, position)
        });
    }

    #[cfg(feature = "egui")]
    fn debug_panel(&mut self, ctx: &egui::Context, frame_index: usize) {
        use egui::widgets::Slider;

        if !self.in_editor() {
            return;
        }

        let render_world = &mut self.render_worlds[frame_index % Self::RENDER_FRAME_COUNT];

        egui::Window::new("World")
            .default_open(true)
            .show(ctx, |ui| {
                ui.heading("Render");
                ui.checkbox(&mut self.render_overlay, "Render overlay");

                ui.heading("Camera");
                ui.checkbox(&mut self.view_debug_camera, "View debug camera");
                ui.checkbox(&mut self.control_debug_camera, "Control debug camera");

                ui.heading("Environment");
                ui.horizontal(|ui| {
                    ui.label("Time of day");
                    ui.add(
                        Slider::new(&mut self.sim_world.time_of_day, 0.0..=24.0)
                            .drag_value_speed(0.01),
                    );
                });

                ui.add(Slider::new(&mut self.shadow_cascades_lambda, 0.0..=1.0));
                ui.collapsing("Ambient", |ui| {
                    ui.add(Slider::new(&mut self.ambient_color.x, 0.0..=1.0));
                    ui.add(Slider::new(&mut self.ambient_color.y, 0.0..=1.0));
                    ui.add(Slider::new(&mut self.ambient_color.z, 0.0..=1.0));
                });

                ui.heading("Terrain");
                self.terrain.debug_panel(ui);

                ui.heading("Geometry Data");
                if let Some(ref geometry_data) = self.geometry_data {
                    ui.label(format!(
                        "position: {:.2}, {:.2}, {:.2}",
                        geometry_data.position.x,
                        geometry_data.position.y,
                        geometry_data.position.z,
                    ));
                    ui.label(format!("id: {:?}", geometry_data.id));
                } else {
                    ui.label("None");
                }

                ui.heading("Entities");
            });

        egui::Window::new("Timings")
            .resizable(false)
            .default_open(false)
            .show(ctx, |ui| {
                ui.set_min_size(egui::Vec2::new(400.0, 300.0));
                let (rect, _resp) =
                    ui.allocate_exact_size(egui::vec2(400.0, 300.0), egui::Sense::hover());

                let painter = ui.painter_at(rect);

                let bar_width = rect.width() / self.fps_history.len() as f32;

                let fps_range = 288.0;

                for i in 0..self.fps_history.len() {
                    let value =
                        self.fps_history[(self.fps_history_cursor + i) % self.fps_history.len()];

                    let left = rect.left() + i as f32 * bar_width;
                    let right = rect.left() + (i + 1) as f32 * bar_width;

                    let bottom = rect.bottom();
                    let top = bottom - rect.height() * ((1.0 / value) / fps_range);

                    let bar_rect = egui::Rect::from_min_max(
                        egui::Pos2::new(left, top),
                        egui::Pos2::new(right, bottom),
                    );
                    painter.rect_filled(bar_rect, 0.0, egui::Color32::RED);
                }

                let line_pos = rect.bottom() - (rect.bottom() - rect.top()) * 0.5;
                painter.line(
                    vec![
                        egui::Pos2::new(rect.left(), line_pos),
                        egui::Pos2::new(rect.right(), line_pos),
                    ],
                    egui::Stroke::new(2.0, egui::Color32::BLUE),
                )
            });

        egui::Window::new("Stats").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Frame index");
                ui.label(format!("{frame_index}"));
            });
            ui.horizontal(|ui| {
                ui.label("Visible chunks");
                ui.label(format!("{}", self.sim_world.visible_chunks.len()));
            });
            ui.horizontal(|ui| {
                ui.label("Visible strata");
                ui.label(format!("{}", render_world.strata_instances.len()));
            });
            ui.horizontal(|ui| {
                ui.label("Gizmo vertices");
                ui.label(format!("{}", render_world.gizmo_vertices.len()));
            });
        });

        self.objects.debug_panel(ctx);
    }
}
