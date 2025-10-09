use crate::{
    engine::{
        prelude::*,
        storage::{Handle, Storage},
    },
    game::math::BoundingSphere,
};

pub struct Object {
    pub transform: Transform,
    pub bounding_sphere: BoundingSphere,
}

#[derive(Default)]
pub struct NewObjects {
    pub objects: Storage<Object>,
}

impl NewObjects {
    pub fn spawn(&mut self, transform: Transform, radius: f32) -> Handle<Object> {
        let bounding_sphere = BoundingSphere {
            center: transform.translation,
            radius,
        };

        self.objects.insert(Object {
            transform,
            bounding_sphere,
        })
    }
}
