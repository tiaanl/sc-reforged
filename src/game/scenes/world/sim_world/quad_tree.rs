use core::f32;

use glam::{IVec2, UVec2, Vec2};

use crate::game::math::{BoundingBox, Containment, Frustum};

pub struct MinMax(pub f32, pub f32);

struct Level {
    size: UVec2,
    min_z: Vec<f32>,
    max_z: Vec<f32>,
}

impl Level {
    #[inline]
    fn index(&self, x: u32, y: u32) -> usize {
        (y as usize) * (self.size.x as usize) + (x as usize)
    }
}

pub struct QuadTree {
    chunk_count: UVec2,
    chunk_size: Vec2,
    levels: Vec<Level>,
}

impl QuadTree {
    pub fn build(chunk_count: UVec2, chunk_size: Vec2, leaves: &[MinMax]) -> QuadTree {
        debug_assert!(
            chunk_count.x > 0 && chunk_count.y > 0,
            "grid can't be empty"
        );

        let area = (chunk_count.x as usize) * (chunk_count.y as usize);
        debug_assert_eq!(leaves.len(), area, "leaf bounds length must be nx * ny");

        let mut levels: Vec<Level> = Vec::default();

        // Build level 0.
        {
            let mut min_z = Vec::with_capacity(area);
            let mut max_z = Vec::with_capacity(area);
            for leaf in leaves {
                min_z.push(leaf.0);
                max_z.push(leaf.1);
            }

            levels.push(Level {
                size: chunk_count,
                min_z,
                max_z,
            });
        }

        while {
            let level = levels.last().unwrap();
            level.size.x > 1 || level.size.y > 1
        } {
            let child = levels.last().unwrap();
            let parent_width = child.size.x.div_ceil(2);
            let parent_height = child.size.y.div_ceil(2);

            let parent_area = (parent_width as usize) * (parent_height as usize);

            let mut parent = Level {
                size: UVec2::new(parent_width, parent_height),
                min_z: vec![f32::INFINITY; parent_area],
                max_z: vec![f32::NEG_INFINITY; parent_area],
            };

            for parent_y in 0..parent_height {
                for parent_x in 0..parent_width {
                    let mut min_z = f32::INFINITY;
                    let mut max_z = f32::NEG_INFINITY;

                    for dy in 0..2 {
                        for dx in 0..2 {
                            let cx = parent_x * 2 + dx;
                            let cy = parent_y * 2 + dy;

                            if cx < child.size.x && cy < child.size.y {
                                let ci = child.index(cx, cy);
                                min_z = min_z.min(child.min_z[ci]);
                                max_z = max_z.max(child.max_z[ci]);
                            }
                        }
                    }

                    let pi = parent.index(parent_x, parent_y);
                    parent.min_z[pi] = min_z;
                    parent.max_z[pi] = max_z;
                }
            }

            levels.push(parent);
        }

        QuadTree {
            chunk_count,
            chunk_size,
            levels,
        }
    }

    #[inline]
    fn node_span_chunks(level: usize) -> u32 {
        1_u32 << level
    }

    pub fn node_bounding_box(&self, level: usize, ix: u32, iy: u32) -> BoundingBox {
        let level_data = &self.levels[level];
        debug_assert!(ix < level_data.size.x && iy < level_data.size.y);

        let i = level_data.index(ix, iy);
        let min_z = level_data.min_z[i];
        let max_z = level_data.max_z[i];

        let span = Self::node_span_chunks(level);

        let min = UVec2::new(ix * span, iy * span);
        let max = (min + UVec2::splat(span)).min(self.chunk_count);

        let min = (min.as_vec2() * self.chunk_size).extend(min_z);
        let max = (max.as_vec2() * self.chunk_size).extend(max_z);

        BoundingBox { min, max }
    }

    #[inline]
    pub fn _root_bounding_box(&self) -> BoundingBox {
        let root_level = self.levels.len() - 1;
        self.node_bounding_box(root_level, 0, 0)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ChunkRect {
    /// inclusive
    pub min: UVec2,
    /// exclusive
    pub max: UVec2,
}

impl QuadTree {
    #[inline]
    fn node_chunk_rect(&self, level: usize, ix: u32, iy: u32) -> ChunkRect {
        let span = Self::node_span_chunks(level);
        let min = UVec2::new(ix * span, iy * span);
        let max = (min + UVec2::splat(span)).min(self.chunk_count);
        ChunkRect { min, max }
    }

    /// Returns visible regions in *chunk coordinates*. This avoids expanding
    /// fully-inside nodes into many leaf chunks.
    fn visible_chunk_rects(&self, frustum: &Frustum, out: &mut Vec<ChunkRect>) {
        out.clear();

        debug_assert!(!self.levels.is_empty(), "how can this be empty?");

        let root_level = self.levels.len() - 1;
        let mut stack: Vec<(usize, u32, u32)> = Vec::default();
        stack.push((root_level, 0, 0));

        while let Some((level, ix, iy)) = stack.pop() {
            let bounding_box = self.node_bounding_box(level, ix, iy);

            match frustum.vs_bounding_box(&bounding_box) {
                Containment::Outside => continue,

                Containment::Intersect => {
                    // Descend if possible; if leaf, accept that single chunk.
                    if level == 0 {
                        // leaf = exactly one chunk.
                        out.push(self.node_chunk_rect(level, ix, iy));
                        continue;
                    }

                    let child_level = level - 1;
                    let child = &self.levels[child_level];

                    for dy in 0..2_u32 {
                        for dx in 0..2_u32 {
                            let cx = ix * 2 + dx;
                            let cy = iy * 2 + dy;
                            if cx < child.size.x && cy < child.size.y {
                                stack.push((child_level, cx, cy));
                            }
                        }
                    }
                }

                Containment::Inside => {
                    // Fully inside: accept whole node as a rect, no need to descend.
                    out.push(self.node_chunk_rect(level, ix, iy));
                }
            }
        }
    }

    pub fn visible_chunks(&self, frustum: &Frustum, out: &mut Vec<IVec2>) {
        // TODO: Don't allocate every time.
        let mut rects = Vec::default();
        self.visible_chunk_rects(frustum, &mut rects);

        out.clear();
        for rect in rects {
            for y in rect.min.y..rect.max.y {
                for x in rect.min.x..rect.max.x {
                    out.push(IVec2::new(x as i32, y as i32));
                }
            }
        }
    }
}
