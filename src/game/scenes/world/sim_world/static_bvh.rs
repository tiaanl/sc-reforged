use glam::{Vec3, Vec4};

use crate::{
    engine::gizmos::{GizmoVertex, create_bounding_box},
    game::math::BoundingBox,
};

enum NodeKind {
    Leaf { start: u32, count: u32 },
    Internal { left: u32, right: u32 },
}

struct Node {
    bounding_box: BoundingBox,
    kind: NodeKind,
}

/// Static binary BVH over object bounding boxes.
pub struct StaticBvh {
    /// Leaf nodes reference ranges in indices.
    nodes: Vec<Node>,
    /// Stores a permutation of object indices.
    indices: Vec<u32>,

    leaf_size: usize,
}

impl StaticBvh {
    pub(crate) fn test(&self, gizmo_vertices: &mut Vec<GizmoVertex>) {
        fn do_node(
            nodes: &[Node],
            index: usize,
            level: usize,
            gizmo_vertices: &mut Vec<GizmoVertex>,
        ) {
            let node = &nodes[index];

            gizmo_vertices.extend(create_bounding_box(
                &node.bounding_box,
                Vec4::new(1.0, 0.0, 0.0, 1.0),
            ));

            match node.kind {
                NodeKind::Leaf { start, count } => {}
                NodeKind::Internal { left, right } => {
                    do_node(nodes, left as usize, level + 1, gizmo_vertices);
                    do_node(nodes, right as usize, level + 1, gizmo_vertices);
                }
            }
        }

        do_node(&self.nodes, 0, 0, gizmo_vertices);
    }

    pub fn new(leaf_size: usize) -> Self {
        Self {
            nodes: Vec::default(),
            indices: Vec::default(),
            leaf_size: leaf_size.max(1),
        }
    }

    pub fn rebuild(&mut self, bounding_boxes: &[BoundingBox]) {
        debug_assert!(!bounding_boxes.is_empty());

        let len = bounding_boxes.len();

        self.nodes = Vec::with_capacity(len * 2);
        self.indices = (0..len as u32).collect();

        self.build_node(bounding_boxes, 0, len);
    }

    fn build_node(&mut self, bounding_boxes: &[BoundingBox], start: usize, count: usize) -> u32 {
        debug_assert!(count > 0);

        let node_index = self.nodes.len() as u32;

        // Placeholder for now.
        self.nodes.push(Node {
            bounding_box: BoundingBox::default(),
            kind: NodeKind::Leaf {
                start: start as u32,
                count: count as u32,
            },
        });

        // Compute bounds over this range.
        let bounding_box = self.range_bounds(bounding_boxes, start, count);

        // Is it a leaf?
        if count <= self.leaf_size {
            self.nodes[node_index as usize] = Node {
                bounding_box,
                kind: NodeKind::Leaf {
                    start: start as u32,
                    count: count as u32,
                },
            };
            return node_index;
        }

        // Choose split axis based on centroid bounds extent.
        let (c_min, c_max) = self.range_centroid_bounds(bounding_boxes, start, count);
        let ext = c_max - c_min;
        let axis = longest_axis(ext);

        // Partition indices by median along the chosen axis.
        let mid = start + count / 2;
        {
            let slice = &mut self.indices[start..start + count];
            let nth = mid - start;
            slice.select_nth_unstable_by(nth, |&a, &b| {
                let ca = bounding_boxes[a as usize].center()[axis];
                let cb = bounding_boxes[b as usize].center()[axis];
                ca.total_cmp(&cb)
            });
        }

        let left_count = mid - start;
        let right_count = count - left_count;

        let left = self.build_node(bounding_boxes, start, left_count);
        let right = self.build_node(bounding_boxes, mid, right_count);

        self.nodes[node_index as usize] = Node {
            bounding_box,
            kind: NodeKind::Internal { left, right },
        };

        node_index
    }

    fn range_bounds(
        &self,
        bounding_boxes: &[BoundingBox],
        start: usize,
        count: usize,
    ) -> BoundingBox {
        let mut result = BoundingBox::default();
        for &i in &self.indices[start..start + count] {
            result.expand_to_include(&bounding_boxes[i as usize]);
        }
        result
    }

    fn range_centroid_bounds(
        &self,
        bounding_boxes: &[BoundingBox],
        start: usize,
        count: usize,
    ) -> (Vec3, Vec3) {
        let mut c_min = Vec3::INFINITY;
        let mut c_max = Vec3::NEG_INFINITY;

        for &i in &self.indices[start..start + count] {
            let c = bounding_boxes[i as usize].center();
            c_min = c_min.min(c);
            c_max = c_max.max(c);
        }

        (c_min, c_max)
    }
}

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
