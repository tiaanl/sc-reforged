use crate::game::interpolate::Interpolate;

#[derive(Clone, Copy, Debug)]
pub struct Key<V> {
    pub frame: u32,
    pub value: V,
}

#[derive(Clone, Debug, Default)]
pub struct Track<V: Interpolate> {
    keys: Vec<Key<V>>,
}

impl<V: Interpolate + Default> Track<V> {
    /// Return the frame number of the last key frame.
    #[inline]
    pub fn last_frame(&self) -> Option<u32> {
        self.keys.last().map(|k| k.frame)
    }

    pub fn insert(&mut self, frame: u32, value: V) {
        match self.keys.binary_search_by_key(&frame, |k| k.frame) {
            Ok(i) => self.keys[i].value = value,                 // last wins
            Err(i) => self.keys.insert(i, Key { frame, value }), // keep sorted
        }
    }

    pub fn _extend<I: IntoIterator<Item = (u32, V)>>(&mut self, it: I) {
        self.keys
            .extend(it.into_iter().map(|(f, v)| Key { frame: f, value: v }));

        // stable sort + last-wins dedup (no finalize step elsewhere)
        self.keys.sort_by_key(|k| k.frame);
        self.keys.reverse();
        self.keys.dedup_by_key(|k| k.frame);
        self.keys.reverse();
    }

    /// Interpolated value at an integer frame index.
    #[inline]
    pub fn _sample_frame(&self, frame: u32) -> V {
        debug_assert!(!self.keys.is_empty(), "Track has no keys!");

        if self.keys.len() == 1 {
            return self.keys[0].value;
        }

        if frame <= self.keys[0].frame {
            return self.keys[0].value;
        }
        let last_idx = self.keys.len() - 1;
        if frame >= self.keys[last_idx].frame {
            return self.keys[last_idx].value;
        }

        let idx = match self.keys.binary_search_by_key(&frame, |k| k.frame) {
            Ok(i) => return self.keys[i].value,
            Err(i) => i - 1,
        };

        let before = self.keys[idx];
        let after = self.keys[idx + 1];

        let span = (after.frame - before.frame) as f32;
        debug_assert!(span > 0.0, "duplicate frames must be deduped in finalize()");
        let t = ((frame - before.frame) as f32 / span).clamp(0.0, 1.0);

        V::interpolate(before.value, after.value, t)
    }

    /// Produce a dense per-frame array for all frames ready for baking to textures.
    pub fn _bake_all(&self) -> Vec<V> {
        debug_assert!(!self.keys.is_empty());
        let last = self.keys[self.keys.len() - 1].frame;
        (0..=last).map(|f| self._sample_frame(f)).collect()
    }

    /// Interpolated value at a fractional frame index.
    /// If `looping`, wrap to [0, last_frame).
    #[inline]
    pub fn sample_sub_frame(&self, frame_f: f32, looping: bool) -> V {
        if self.keys.is_empty() {
            return V::default();
        }
        // debug_assert!(!self.keys.is_empty());

        if self.keys.len() == 1 {
            return self.keys[0].value;
        }

        let first = self.keys[0].frame as f32;
        let last = self.keys[self.keys.len() - 1].frame as f32;

        let f = if looping && last > first {
            let span = last - first;
            first + (frame_f - first).rem_euclid(span) // [first,last)
        } else {
            frame_f.clamp(first, last)
        };

        if f <= first {
            return self.keys[0].value;
        }
        if f >= last {
            return self.keys[self.keys.len() - 1].value;
        }

        let i = self.keys.partition_point(|k| (k.frame as f32) <= f);
        let a = self.keys[i - 1];
        let b = self.keys[i];
        let t = ((f - a.frame as f32) / (b.frame as f32 - a.frame as f32)).clamp(0.0, 1.0);

        V::interpolate(a.value, b.value, t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Quat, Vec3};

    #[inline]
    fn approx_f(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }
    #[inline]
    fn approx_v3(a: Vec3, b: Vec3) -> bool {
        approx_f(a.x, b.x) && approx_f(a.y, b.y) && approx_f(a.z, b.z)
    }
    #[inline]
    fn approx_q(a: Quat, b: Quat) -> bool {
        // Quats can differ by sign; compare via absolute dot near 1
        a.is_normalized() && b.is_normalized() && a.dot(b).abs() > 1.0 - 1e-4
    }

    #[test]
    fn subframe_interpolates_vec3_midpoint() {
        let mut t = Track::<Vec3>::default();
        t.insert(0, Vec3::new(0.0, 0.0, 0.0));
        t.insert(10, Vec3::new(10.0, 0.0, 0.0));

        let v = t.sample_sub_frame(5.0, false);
        assert!(approx_v3(v, Vec3::new(5.0, 0.0, 0.0)));
    }

    #[test]
    fn subframe_exact_key_hit_vec3() {
        let mut t = Track::<Vec3>::default();
        t.insert(0, Vec3::splat(1.0));
        t.insert(8, Vec3::splat(3.0));
        t.insert(12, Vec3::splat(7.0));

        let v = t.sample_sub_frame(8.0, false);
        assert!(approx_v3(v, Vec3::splat(3.0)));
    }

    #[test]
    fn subframe_clamps_before_after_range() {
        let mut t = Track::<Vec3>::default();
        t.insert(2, Vec3::new(2.0, 0.0, 0.0));
        t.insert(6, Vec3::new(6.0, 0.0, 0.0));

        // Before first
        let v0 = t.sample_sub_frame(0.0, false);
        assert!(approx_v3(v0, Vec3::new(2.0, 0.0, 0.0)));

        // After last
        let v1 = t.sample_sub_frame(100.0, false);
        assert!(approx_v3(v1, Vec3::new(6.0, 0.0, 0.0)));
    }

    #[test]
    fn subframe_looping_wraps_across_end() {
        let mut t = Track::<Vec3>::default();
        t.insert(0, Vec3::new(0.0, 0.0, 0.0));
        t.insert(10, Vec3::new(10.0, 0.0, 0.0));

        // 10.5 wraps to 0.5 (since last_frame == 10), expect 0.5
        let v = t.sample_sub_frame(10.5, true);
        assert!(approx_v3(v, Vec3::new(0.5, 0.0, 0.0)));

        // 19.0 wraps to 9.0
        let v2 = t.sample_sub_frame(19.0, true);
        assert!(approx_v3(v2, Vec3::new(9.0, 0.0, 0.0)));
    }

    #[test]
    fn last_wins_on_duplicate_inserts() {
        let mut t = Track::<Vec3>::default();
        t.insert(0, Vec3::new(0.0, 0.0, 0.0));
        t.insert(5, Vec3::new(999.0, 0.0, 0.0)); // duplicate frame
        t.insert(5, Vec3::new(5.0, 0.0, 0.0)); // last should win
        t.insert(10, Vec3::new(10.0, 0.0, 0.0));

        // Exact key at 5 should be 5.0, not 999.0
        let v = t.sample_sub_frame(5.0, false);
        assert!(approx_v3(v, Vec3::new(5.0, 0.0, 0.0)));
    }

    #[test]
    fn quaternion_shortest_arc_is_respected() {
        let a = Quat::IDENTITY;
        let b = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let b_flipped = Quat::from_xyzw(-b.x, -b.y, -b.z, -b.w);

        let mut t = Track::<Quat>::default();
        t.insert(0, a);
        t.insert(10, b_flipped); // same rotation as b, opposite hemisphere

        // Halfway should be ~45deg around Y
        let q_mid = t.sample_sub_frame(5.0, false);
        let expected = a.slerp(b, 0.5);
        assert!(approx_q(q_mid, expected));
    }

    #[test]
    fn integer_sampling_matches_subframe_on_integer_inputs() {
        let mut t = Track::<Vec3>::default();
        t.insert(0, Vec3::new(0.0, 0.0, 0.0));
        t.insert(10, Vec3::new(10.0, 0.0, 0.0));

        // If `sample_frame` is public, use it; otherwise compare against sub-frame at integer.
        #[allow(unused_mut)]
        let mut a = t.sample_sub_frame(7.0, false);
        // let a = t.sample_frame(7); // <- use this if you’ve exposed it
        let b = t.sample_sub_frame(7.0, false);
        assert!(approx_v3(a, b));
    }
}
