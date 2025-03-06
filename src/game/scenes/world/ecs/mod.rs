pub use bevy_ecs::prelude as ecs;
use bevy_ecs::schedule::ScheduleLabel;

mod gizmos;
mod transform;

pub use gizmos::*;
pub use transform::*;

#[derive(Clone, Debug, Eq, Hash, PartialEq, ScheduleLabel)]
struct UpdateSchedule;

#[derive(Clone, Debug, Eq, Hash, PartialEq, ScheduleLabel)]
struct RenderSchedule;

pub struct World {
    pub world: ecs::World,
    pub update_schedule: ecs::Schedule,
    pub render_schedule: ecs::Schedule,
}

impl World {
    pub fn new() -> Self {
        let mut world = ecs::World::new();
        let mut update_schedule = ecs::Schedule::new(UpdateSchedule);
        let mut render_schedule = ecs::Schedule::new(RenderSchedule);

        update_schedule.add_systems(update);

        gizmos::setup_ecs(&mut world, &mut render_schedule);

        Self {
            world,
            update_schedule,
            render_schedule,
        }
    }

    pub fn update(&mut self) {
        self.update_schedule.run(&mut self.world);
    }

    pub fn prepare_render(&mut self) {
        self.render_schedule.run(&mut self.world);
    }

    pub fn spawn_object(&mut self, transform: Transform) {
        self.world.spawn((transform, EntityGizmos::default()));
    }
}

fn update() {}
