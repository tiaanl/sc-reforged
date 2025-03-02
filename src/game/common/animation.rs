use glam::{Quat, Vec3};

pub trait Interpolate: Clone {
    fn interpolate(left: &Self, right: &Self, n: f32) -> Self;
}

impl Interpolate for f32 {
    #[inline]
    fn interpolate(left: &Self, right: &Self, n: f32) -> Self {
        left + (right - left) * n
    }
}

impl Interpolate for Vec3 {
    #[inline]
    fn interpolate(left: &Self, right: &Self, n: f32) -> Self {
        left.lerp(*right, n)
    }
}

impl Interpolate for Quat {
    fn interpolate(left: &Self, right: &Self, n: f32) -> Self {
        left.slerp(*right, n)
    }
}

impl<A: Interpolate, B: Interpolate> Interpolate for (A, B) {
    fn interpolate(left: &Self, right: &Self, n: f32) -> Self {
        (
            A::interpolate(&left.0, &right.0, n),
            B::interpolate(&left.1, &right.1, n),
        )
    }
}

pub struct KeyFrame<V: Interpolate> {
    time: f32,
    value: V,
}

#[derive(Default)]
pub struct Track<V: Interpolate> {
    key_frames: Vec<KeyFrame<V>>,
}

impl<V: Interpolate> Track<V> {
    pub fn set_key_frame(&mut self, time: f32, value: V) {
        let pos = self
            .key_frames
            .binary_search_by(|key_frame| key_frame.time.partial_cmp(&time).unwrap())
            .unwrap_or_else(|e| e);
        self.key_frames.insert(pos, KeyFrame { time, value });
    }

    pub fn get(&self, time: f32) -> V {
        let len = self.key_frames.len();
        if len == 0 {
            panic!("No keyframes in track");
        }

        if time <= self.key_frames[0].time {
            return self.key_frames[0].value.clone();
        }

        if time >= self.key_frames[len - 1].time {
            return self.key_frames[len - 1].value.clone();
        }

        for window in self.key_frames.windows(2) {
            let (left, right) = (&window[0], &window[1]);
            if time >= left.time && time <= right.time {
                let t = (time - left.time) / (right.time - left.time);
                return V::interpolate(&left.value, &right.value, t);
            }
        }

        unreachable!()
    }
}
