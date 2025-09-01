use glam::{IVec2, Vec3, Vec4};
use nalgebra as na;
use rapier3d::prelude::*;

use crate::{
    engine::{gizmos::GizmoVertex, prelude::Transform, storage::Handle},
    game::{height_map::HeightMap, model::Model, models::models},
};

impl From<Transform> for na::Isometry3<f32> {
    fn from(value: Transform) -> Self {
        let translation = na::Translation3::new(
            value.translation.x,
            value.translation.y,
            value.translation.z,
        );
        let rotation = na::UnitQuaternion::from_quaternion(na::Quaternion::new(
            value.rotation.w,
            value.rotation.x,
            value.rotation.y,
            value.rotation.z,
        ));

        na::Isometry3::from_parts(translation, rotation)
    }
}

pub struct Physics {
    gravity: Vec3,
    integration_parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: DefaultBroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,

    pub rigid_body_set: RigidBodySet,
    collider_set: ColliderSet,

    debug: DebugRenderPipeline,
}

impl Physics {
    pub fn new() -> Self {
        let gravity = Vec3::new(0.0, 0.0, -9.81);
        let integration_parameters = IntegrationParameters::default();
        let physics_pipeline = PhysicsPipeline::new();
        let island_manager = IslandManager::new();
        let broad_phase = DefaultBroadPhase::new();
        let narrow_phase = NarrowPhase::new();
        let impulse_joint_set = ImpulseJointSet::new();
        let multibody_joint_set = MultibodyJointSet::new();
        let ccd_solver = CCDSolver::new();

        let rigid_body_set = RigidBodySet::new();
        let collider_set = ColliderSet::new();

        let debug = DebugRenderPipeline::new(
            DebugRenderStyle {
                rigid_body_axes_length: 50.0,
                ..Default::default()
            },
            DebugRenderMode::all(),
        );

        Self {
            gravity,
            integration_parameters,
            physics_pipeline,
            island_manager,
            broad_phase,
            narrow_phase,
            impulse_joint_set,
            multibody_joint_set,
            ccd_solver,
            rigid_body_set,
            collider_set,

            debug,
        }
    }

    pub fn debug_render(&mut self, gizmo_vertices: &mut Vec<GizmoVertex>) {
        let mut m = DebugRenderer(gizmo_vertices);
        self.debug.render(
            &mut m,
            &self.rigid_body_set,
            &self.collider_set,
            &self.impulse_joint_set,
            &self.multibody_joint_set,
            &self.narrow_phase,
        )
    }

    pub fn get_rigid_body(&self, handle: RigidBodyHandle) -> Option<&RigidBody> {
        self.rigid_body_set.get(handle)
    }

    pub fn insert_terrain_collider(
        &mut self,
        hm: &HeightMap,
        nominal_edge_size: f32,
        altitude_map_height_base: f32,
    ) -> (RigidBodyHandle, ColliderHandle) {
        let nx = hm.size.x as usize;
        let nz = hm.size.y as usize;

        let heights = na::DMatrix::from_fn(nz + 1, nx + 1, |r, c| {
            (u8::MAX - hm.elevation_at(IVec2::new(c as i32, r as i32))) as f32
        });

        // World extents (the field spans exactly these lengths horizontally).
        let world_len_x = nx as f32 * nominal_edge_size;
        let world_len_y = nz as f32 * nominal_edge_size;

        // Fixed body for the terrain
        let rb = self
            .rigid_body_set
            .insert(RigidBodyBuilder::fixed().build());

        // Map Rapier's local Y-up to world Z-up.
        let rot_x = na::UnitQuaternion::from_axis_angle(
            &na::Vector3::x_axis(),
            -std::f32::consts::FRAC_PI_2,
        );

        let iso = na::Isometry3::from_parts(
            na::Translation3::new(
                world_len_x * 0.5,
                world_len_y * 0.5,
                255.0 * altitude_map_height_base,
            ),
            rot_x,
        );

        let scale = na::Vector3::new(world_len_x, altitude_map_height_base, world_len_y);

        let col = ColliderBuilder::heightfield(heights, scale)
            .position(iso)
            .friction(0.9)
            .restitution(0.0)
            .build();

        let ch = self
            .collider_set
            .insert_with_parent(col, rb, &mut self.rigid_body_set);

        (rb, ch)
    }

    pub fn insert_object(&mut self, transform: Transform, is_dynamic: bool) -> RigidBodyHandle {
        let position: na::Isometry3<_> = transform.into();

        self.rigid_body_set.insert(
            RigidBodyBuilder::new(if is_dynamic {
                RigidBodyType::Dynamic
            } else {
                RigidBodyType::Fixed
            })
            .position(position)
            .build(),
        )
    }

    pub fn attach_model(
        &mut self,
        handle: RigidBodyHandle,
        model: Handle<Model>,
    ) -> Vec<ColliderHandle> {
        const SCALE: f32 = 1.0;

        let model = models().get(model).expect("Missing model!");

        let mut handles = Vec::with_capacity(model.collision_boxes.len());

        for collision_box in model.collision_boxes.iter() {
            let (center, half) = collision_box.center_and_half_extent();
            let center = center * SCALE;
            let half = half * SCALE;

            // Local offset of collider relative to the body's CoM: node * box-center
            let local = model
                .skeleton
                .local_transform(collision_box.node_index)
                .transform_point3(center);

            let shape = SharedShape::cuboid(half.x, half.y, half.z);
            let collider = ColliderBuilder::new(shape).position(local.into()).build();

            let collider_handle =
                self.collider_set
                    .insert_with_parent(collider, handle, &mut self.rigid_body_set);

            handles.push(collider_handle);
        }

        handles
    }

    pub fn step(&mut self) {
        self.physics_pipeline.step(
            &self.gravity.into(),
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            &(),
            &(),
        );

        // for pair in self.narrow_phase.contact_pairs() {
        //     println!("Contact: {:?} <-> {:?}", pair.collider1, pair.collider2);
        // }
    }
}

struct DebugRenderer<'a>(&'a mut Vec<GizmoVertex>);

impl<'a> DebugRenderBackend for DebugRenderer<'a> {
    fn draw_line(
        &mut self,
        _object: DebugRenderObject,
        a: Point<f32>,
        b: Point<f32>,
        color: DebugColor,
    ) {
        let color = Vec4::new(color[0], color[1], color[2], color[3]);

        self.0
            .push(GizmoVertex::new(Vec3::new(a.x, a.y, a.z), color));
        self.0
            .push(GizmoVertex::new(Vec3::new(b.x, b.y, b.z), color));
    }
}
