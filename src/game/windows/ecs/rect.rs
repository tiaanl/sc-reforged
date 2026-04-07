use bevy_ecs::prelude::*;
use glam::{IVec2, UVec2};

#[derive(Component, Default)]
#[require(GlobalRect)]
pub struct Rect {
    pub position: IVec2,
    pub size: UVec2,
}

impl Rect {
    pub const UI: Rect = Rect::new(IVec2::ZERO, UVec2::new(640, 480));

    pub const fn new(position: IVec2, size: UVec2) -> Self {
        Self { position, size }
    }
}

#[derive(Component, Default)]
pub struct GlobalRect {
    pub min: IVec2,
    pub max: IVec2,
}

/// Recomputes global rectangles by accumulating each entity's local rect
/// position through the `ChildOf` hierarchy.
// TODO: Add change detection.
pub fn update_global_rects(
    roots: Query<Entity, (With<Rect>, Without<ChildOf>)>,
    rects: Query<(&Rect, Option<&Children>)>,
    mut global_rects: Query<&mut GlobalRect>,
) {
    for root in roots.iter() {
        update_entity_global_rect(root, IVec2::ZERO, &rects, &mut global_rects);
    }
}

fn update_entity_global_rect(
    entity: Entity,
    parent_min: IVec2,
    rects: &Query<(&Rect, Option<&Children>)>,
    global_rects: &mut Query<&mut GlobalRect>,
) {
    let Ok((rect, children)) = rects.get(entity) else {
        return;
    };

    let min = parent_min + rect.position;
    let max = min + rect.size.as_ivec2();

    if let Ok(mut global_rect) = global_rects.get_mut(entity) {
        global_rect.min = min;
        global_rect.max = max;
    }

    if let Some(children) = children {
        for &child in children {
            update_entity_global_rect(child, min, rects, global_rects);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::{schedule::Schedule, world::World};

    #[test]
    fn update_global_rects_accumulates_parent_offsets() {
        let mut world = World::new();

        let grandchild = world
            .spawn(Rect::new(IVec2::new(-2, 7), UVec2::new(4, 6)))
            .id();

        let child = world
            .spawn(Rect::new(IVec2::new(5, -3), UVec2::new(10, 10)))
            .add_child(grandchild)
            .id();

        let root = world
            .spawn(Rect::new(IVec2::new(10, 20), UVec2::new(100, 50)))
            .add_child(child)
            .id();

        let mut schedule = Schedule::default();
        schedule.add_systems(update_global_rects);
        schedule.run(&mut world);

        let root_rect = world.entity(root).get::<GlobalRect>().unwrap();
        assert_eq!(root_rect.min, IVec2::new(10, 20));
        assert_eq!(root_rect.max, IVec2::new(110, 70));

        let child_rect = world.entity(child).get::<GlobalRect>().unwrap();
        assert_eq!(child_rect.min, IVec2::new(15, 17));
        assert_eq!(child_rect.max, IVec2::new(25, 27));

        let grandchild_rect = world.entity(grandchild).get::<GlobalRect>().unwrap();
        assert_eq!(grandchild_rect.min, IVec2::new(13, 24));
        assert_eq!(grandchild_rect.max, IVec2::new(17, 30));
    }
}
