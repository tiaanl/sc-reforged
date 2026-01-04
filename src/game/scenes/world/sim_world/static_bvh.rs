use glam::Vec3;

use crate::game::math::{BoundingBox, Containment, Frustum, RaySegment};

enum NodeKind {
    Leaf { start: usize, count: usize },
    Internal { left: usize, right: usize },
}

struct Node {
    bounding_box: BoundingBox,
    kind: NodeKind,
}

/// Static binary BVH over object bounding boxes.
pub struct StaticBvh<T: Copy> {
    nodes: Vec<Node>,
    indices: Vec<usize>,

    objects: Vec<T>,
    bounding_boxes: Vec<BoundingBox>,

    leaf_size: usize,
}

impl<T: Copy> StaticBvh<T> {
    pub fn new(leaf_size: usize) -> Self {
        Self {
            nodes: Vec::new(),
            indices: Vec::new(),
            objects: Vec::new(),
            bounding_boxes: Vec::new(),
            leaf_size: leaf_size.max(1),
        }
    }

    /// Rebuild the nodes with the given items.
    pub fn rebuild(&mut self, items: &[(T, BoundingBox)]) {
        debug_assert!(!items.is_empty());

        let len = items.len();

        self.objects.clear();
        self.objects.reserve(len);

        self.bounding_boxes.clear();
        self.bounding_boxes.reserve(len);

        for (id, bounding_box) in items.iter() {
            self.objects.push(*id);
            self.bounding_boxes.push(*bounding_box);
        }

        self.nodes = Vec::with_capacity(len * 2);
        self.indices = (0..len).collect();

        self.build_node(0, len);
    }

    /// Frustum culling query. Writes visible object IDs into `out`.
    pub fn objects_in_frustum(&self, frustum: &Frustum, out: &mut Vec<T>) {
        out.clear();

        if self.nodes.is_empty() {
            return;
        }

        // (node_index, parent_fully_inside)
        let mut stack: Vec<(usize, bool)> = Vec::new();
        stack.push((0, false));

        while let Some((node_index, parent_inside)) = stack.pop() {
            let node = &self.nodes[node_index];

            if parent_inside {
                match node.kind {
                    NodeKind::Leaf { start, count } => {
                        for &item_index in &self.indices[start..start + count] {
                            out.push(self.objects[item_index]);
                        }
                    }
                    NodeKind::Internal { left, right } => {
                        stack.push((left, true));
                        stack.push((right, true));
                    }
                }
                continue;
            }

            match frustum.vs_bounding_box(&node.bounding_box) {
                Containment::Outside => {}
                Containment::Inside => match node.kind {
                    NodeKind::Leaf { start, count } => {
                        for &item_index in &self.indices[start..start + count] {
                            out.push(self.objects[item_index]);
                        }
                    }
                    NodeKind::Internal { left, right } => {
                        stack.push((left, true));
                        stack.push((right, true));
                    }
                },
                Containment::Intersect => match node.kind {
                    NodeKind::Leaf { start, count } => {
                        for &item_i in &self.indices[start..start + count] {
                            if frustum.intersects_bounding_box(&self.bounding_boxes[item_i]) {
                                out.push(self.objects[item_i]);
                            }
                        }
                    }
                    NodeKind::Internal { left, right } => {
                        stack.push((left, false));
                        stack.push((right, false));
                    }
                },
            }
        }
    }

    /// Ray query. Writes object IDs whose bounding boxes intersect the ray segment into `out`.
    pub fn _objects_intersect_ray_segment(&self, ray_segment: &RaySegment, out: &mut Vec<T>) {
        out.clear();

        if self.nodes.is_empty() || ray_segment.is_degenerate() {
            return;
        }

        let mut stack: Vec<usize> = Vec::new();
        stack.push(0);

        while let Some(node_index) = stack.pop() {
            let node = &self.nodes[node_index];
            if node
                .bounding_box
                .intersect_ray_segment(ray_segment)
                .is_none()
            {
                continue;
            }

            match node.kind {
                NodeKind::Leaf { start, count } => {
                    for &item_index in self.indices[start..start + count].iter() {
                        if self.bounding_boxes[item_index]
                            .intersect_ray_segment(ray_segment)
                            .is_some()
                        {
                            out.push(self.objects[item_index]);
                        }
                    }
                }
                NodeKind::Internal { left, right } => {
                    stack.push(left);
                    stack.push(right);
                }
            }
        }
    }

    fn build_node(&mut self, start: usize, count: usize) -> usize {
        debug_assert!(count > 0);

        let node_index = self.nodes.len();

        self.nodes.push(Node {
            bounding_box: BoundingBox::default(),
            kind: NodeKind::Leaf { start, count },
        });

        // Compute bounds over this range.
        let bounding_box = self.range_bounds(start, count);

        // Is it a leaf?
        if count <= self.leaf_size {
            self.nodes[node_index] = Node {
                bounding_box,
                kind: NodeKind::Leaf { start, count },
            };
            return node_index;
        }

        // Split axis based on centroid bounds extent.
        let axis = {
            #[inline]
            fn longest_axis(v: Vec3) -> usize {
                let ax = v.x.abs();
                let ay = v.y.abs();
                let az = v.z.abs();
                if ax >= ay && ax >= az {
                    0
                } else if ay >= az {
                    1
                } else {
                    2
                }
            }

            let (c_min, c_max) = self.range_centroid_bounds(start, count);
            longest_axis(c_max - c_min)
        };

        // Partition indices by median centroid along that axis.
        let mid = start + count / 2;
        {
            let slice = &mut self.indices[start..start + count];
            let nth = mid - start;

            slice.select_nth_unstable_by(nth, |&a, &b| {
                let ca = self.bounding_boxes[a].center()[axis];
                let cb = self.bounding_boxes[b].center()[axis];
                ca.total_cmp(&cb)
            });
        }

        let left_count = mid - start;
        let right_count = count - left_count;

        let left = self.build_node(start, left_count);
        let right = self.build_node(mid, right_count);

        self.nodes[node_index] = Node {
            bounding_box,
            kind: NodeKind::Internal { left, right },
        };

        node_index
    }

    fn range_bounds(&self, start: usize, count: usize) -> BoundingBox {
        debug_assert!(count > 0);

        let mut result = BoundingBox::default();
        for &i in &self.indices[start..start + count] {
            result.expand_to_include(&self.bounding_boxes[i]);
        }
        result
    }

    fn range_centroid_bounds(&self, start: usize, count: usize) -> (Vec3, Vec3) {
        debug_assert!(count > 0);

        let mut c_min = Vec3::INFINITY;
        let mut c_max = Vec3::NEG_INFINITY;

        for &i in &self.indices[start..start + count] {
            let c = self.bounding_boxes[i].center();
            c_min = c_min.min(c);
            c_max = c_max.max(c);
        }

        (c_min, c_max)
    }
}
