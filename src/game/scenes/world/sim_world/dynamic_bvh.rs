use bevy_ecs::prelude::*;
use generational_arena::{Arena, Index as Handle};
use glam::Vec3;

use crate::game::math::{BoundingBox, Containment, Frustum, RaySegment};

#[derive(Clone, Copy, Component)]
pub struct DynamicBvhHandle(Handle);

/// Node stored in the arena. Internal nodes have two children; leaf nodes have
/// an object payload.
#[derive(Clone)]
struct Node {
    bounding_box: BoundingBox,
    parent: Option<Handle>,
    kind: NodeKind,
}

#[derive(Clone)]
enum NodeKind {
    Internal {
        child1: Handle,
        child2: Handle,
        height: i32,
    },
    Leaf {
        object: Entity,
    },
}

/// Dynamic BVH tree.
#[derive(Resource)]
pub struct DynamicBvh {
    nodes: Arena<Node>,
    root: Option<Handle>,

    fat_margin: f32,
    velocity_inflate: f32,
}

impl DynamicBvh {
    /// Insert object with its *tight* AABB. Returns a handle you store externally.
    pub fn insert(&mut self, object: Entity, tight: BoundingBox) -> DynamicBvhHandle {
        let fat = self.fatten_aabb(tight, Vec3::ZERO);

        let leaf = self.nodes.insert(Node {
            bounding_box: fat,
            parent: None,
            kind: NodeKind::Leaf { object },
        });

        self.insert_leaf(leaf);

        DynamicBvhHandle(leaf)
    }

    /// Remove a previously inserted handle.
    pub fn _remove(&mut self, handle: DynamicBvhHandle) -> Option<Entity> {
        let handle = handle.0;

        if !self.nodes.contains(handle) {
            return None;
        }

        self.remove_leaf(handle);

        // Take the object out and remove the node from arena.
        let obj = match self.nodes.get(handle) {
            Some(Node {
                kind: NodeKind::Leaf { object },
                ..
            }) => Some(*object),
            _ => None,
        };
        let _ = self.nodes.remove(handle);
        obj
    }

    /// Update an object's tight AABB.
    ///
    /// `displacement` is (new_center - old_center) or velocity*dt; used to inflate fat AABB so
    /// fast movers don’t thrash reinserts.
    ///
    /// Returns true if the proxy was reinserted (tree topology changed).
    pub fn update(
        &mut self,
        handle: DynamicBvhHandle,
        new_tight: BoundingBox,
        displacement: Vec3,
    ) -> bool {
        let handle = handle.0;

        if !self.nodes.contains(handle) {
            return false;
        }

        // If still contained in current fat box, do nothing.
        if self.nodes[handle].bounding_box.contains_aabb(&new_tight) {
            return false;
        }

        // Otherwise: remove + reinsert with a new fattened AABB.
        self.remove_leaf(handle);

        let new_fat = self.fatten_aabb(new_tight, displacement);
        self.nodes[handle].bounding_box = new_fat;

        self.insert_leaf(handle);
        true
    }

    // --------------------
    // Queries
    // --------------------

    pub fn query_frustum(&self, frustum: &Frustum, out: &mut Vec<Entity>) {
        let Some(root) = self.root else {
            return;
        };

        // (node, parent_fully_inside)
        let mut stack: Vec<(Handle, bool)> = Vec::new();
        stack.push((root, false));

        while let Some((h, parent_inside)) = stack.pop() {
            let node = &self.nodes[h];

            if parent_inside {
                match node.kind {
                    NodeKind::Leaf { object } => {
                        out.push(object);
                    }
                    NodeKind::Internal { child1, child2, .. } => {
                        stack.push((child1, true));
                        stack.push((child2, true));
                    }
                }
                continue;
            }

            match frustum.vs_bounding_box(&node.bounding_box) {
                Containment::Outside => {}
                Containment::Inside => match node.kind {
                    NodeKind::Leaf { object } => {
                        out.push(object);
                    }
                    NodeKind::Internal { child1, child2, .. } => {
                        stack.push((child1, true));
                        stack.push((child2, true));
                    }
                },
                Containment::Intersect => {
                    match node.kind {
                        NodeKind::Leaf { object } => {
                            // If you want extra precision you can keep tight boxes externally and test them here.
                            out.push(object);
                        }
                        NodeKind::Internal { child1, child2, .. } => {
                            stack.push((child1, false));
                            stack.push((child2, false));
                        }
                    }
                }
            }
        }
    }

    pub fn _query_ray_segment(&self, ray: &RaySegment, out: &mut Vec<Entity>) {
        let Some(root) = self.root else {
            return;
        };
        if ray.is_degenerate() {
            return;
        }

        let mut stack: Vec<Handle> = Vec::new();
        stack.push(root);

        while let Some(h) = stack.pop() {
            let node = &self.nodes[h];
            if node.bounding_box.intersect_ray_segment(ray).is_none() {
                continue;
            }

            match node.kind {
                NodeKind::Leaf { object } => {
                    out.push(object);
                }
                NodeKind::Internal { child1, child2, .. } => {
                    stack.push(child1);
                    stack.push(child2);
                }
            }
        }
    }

    // --------------------
    // Internals
    // --------------------

    fn fatten_aabb(&self, tight: BoundingBox, displacement: Vec3) -> BoundingBox {
        if self.fat_margin == 0.0 && self.velocity_inflate == 0.0 {
            return tight;
        }

        let mut fat = tight;

        if self.fat_margin != 0.0 {
            fat.expand(Vec3::splat(self.fat_margin));
        }

        if self.velocity_inflate != 0.0 {
            fat.expand(displacement.abs() * self.velocity_inflate);
        }

        fat
    }

    fn insert_leaf(&mut self, leaf: Handle) {
        // Empty tree.
        if self.root.is_none() {
            self.root = Some(leaf);
            self.nodes[leaf].parent = None;
            return;
        }

        #[inline]
        fn surface_area(bb: &BoundingBox) -> f32 {
            let d = bb.max - bb.min;
            // If you prefer "perimeter" like Box2D 2D, for 3D use surface area.
            2.0 * (d.x * d.y + d.y * d.z + d.z * d.x)
        }

        // ----------------------------
        // 1) Find the best sibling
        // ----------------------------
        let leaf_aabb = self.nodes[leaf].bounding_box;

        let mut index = self.root.unwrap();
        while !matches!(self.nodes[index].kind, NodeKind::Leaf { .. }) {
            let (c1, c2) = match self.nodes[index].kind {
                NodeKind::Internal { child1, child2, .. } => (child1, child2),
                NodeKind::Leaf { .. } => unreachable!("leaf in internal-only traversal"),
            };

            let cost1 = {
                let u = leaf_aabb.union(&self.nodes[c1].bounding_box);
                surface_area(&u) - surface_area(&self.nodes[c1].bounding_box)
            };

            let cost2 = {
                let u = leaf_aabb.union(&self.nodes[c2].bounding_box);
                surface_area(&u) - surface_area(&self.nodes[c2].bounding_box)
            };

            // Descend toward cheaper child.
            index = if cost1 < cost2 { c1 } else { c2 };
        }

        let sibling = index;

        // ----------------------------
        // 2) Create a new parent for (sibling, leaf)
        // ----------------------------
        let old_parent = self.nodes[sibling].parent;

        let new_parent_aabb = leaf_aabb.union(&self.nodes[sibling].bounding_box);

        let sibling_height = match self.nodes[sibling].kind {
            NodeKind::Internal { height, .. } => height,
            NodeKind::Leaf { .. } => 0,
        };
        let leaf_height = match self.nodes[leaf].kind {
            NodeKind::Internal { height, .. } => height,
            NodeKind::Leaf { .. } => 0,
        };

        let new_parent = self.nodes.insert(Node {
            bounding_box: new_parent_aabb,
            parent: old_parent,
            kind: NodeKind::Internal {
                child1: sibling,
                child2: leaf,
                height: sibling_height.max(leaf_height) + 1,
            },
        });

        // Fix parent pointers on children.
        self.nodes[sibling].parent = Some(new_parent);
        self.nodes[leaf].parent = Some(new_parent);

        // ----------------------------
        // 3) Hook new parent into the old parent (or become root)
        // ----------------------------
        if let Some(p) = old_parent {
            let (p_c1, p_c2) = match self.nodes[p].kind {
                NodeKind::Internal { child1, child2, .. } => (child1, child2),
                NodeKind::Leaf { .. } => unreachable!("parent must be internal"),
            };

            if p_c1 == sibling {
                if let NodeKind::Internal { ref mut child1, .. } = self.nodes[p].kind {
                    *child1 = new_parent;
                }
            } else {
                debug_assert!(p_c2 == sibling);
                if let NodeKind::Internal { ref mut child2, .. } = self.nodes[p].kind {
                    *child2 = new_parent;
                }
            }
        } else {
            // Sibling was root.
            self.root = Some(new_parent);
        }

        // ----------------------------
        // 4) Walk upward: refit bounds + (optional) balance
        // ----------------------------
        // If you implement balance() to return the new subtree root after rotations,
        // you’ll want to integrate it into refit_upwards. For now we just refit.
        self.refit_upwards(Some(new_parent));
    }

    fn remove_leaf(&mut self, leaf: Handle) {
        // If leaf is root, tree becomes empty.
        if self.root == Some(leaf) {
            self.root = None;
            self.nodes[leaf].parent = None;
            return;
        }

        let parent = self.nodes[leaf]
            .parent
            .expect("leaf must have a parent unless it is root");

        let grandparent = self.nodes[parent].parent;

        // Identify sibling (the other child of parent).
        let (c1, c2) = match self.nodes[parent].kind {
            NodeKind::Internal { child1, child2, .. } => (child1, child2),
            NodeKind::Leaf { .. } => unreachable!("parent must be internal"),
        };

        let sibling = if c1 == leaf {
            c2
        } else {
            debug_assert!(c2 == leaf);
            c1
        };

        // Splice sibling up into grandparent.
        if let Some(gp) = grandparent {
            // Replace `parent` in gp's children with `sibling`.
            let (gp_c1, gp_c2) = match self.nodes[gp].kind {
                NodeKind::Internal { child1, child2, .. } => (child1, child2),
                NodeKind::Leaf { .. } => unreachable!("grandparent must be internal"),
            };

            if gp_c1 == parent {
                if let NodeKind::Internal { ref mut child1, .. } = self.nodes[gp].kind {
                    *child1 = sibling;
                }
            } else {
                debug_assert!(gp_c2 == parent);
                if let NodeKind::Internal { ref mut child2, .. } = self.nodes[gp].kind {
                    *child2 = sibling;
                }
            }

            self.nodes[sibling].parent = Some(gp);

            // Remove parent node from arena (it’s now unreachable).
            // Important: parent has no payload; we can discard it.
            let _ = self.nodes.remove(parent);

            // Leaf is now detached.
            self.nodes[leaf].parent = None;

            // Refit bounds/heights up the tree.
            self.refit_upwards(Some(gp));
        } else {
            // Parent was root: sibling becomes the new root.
            self.root = Some(sibling);
            self.nodes[sibling].parent = None;

            let _ = self.nodes.remove(parent);

            // Leaf is now detached.
            self.nodes[leaf].parent = None;
        }
    }

    fn refit_upwards(&mut self, mut h: Option<Handle>) {
        // Recompute AABBs and heights while walking to root.
        while let Some(node_h) = h {
            let (new_aabb, new_height, _parent, is_leaf) = {
                let node = &self.nodes[node_h];

                match node.kind {
                    NodeKind::Leaf { .. } => (node.bounding_box, 0, node.parent, true),
                    NodeKind::Internal { child1, child2, .. } => {
                        let aabb = self.nodes[child1]
                            .bounding_box
                            .union(&self.nodes[child2].bounding_box);
                        let h1 = match self.nodes[child1].kind {
                            NodeKind::Internal { height, .. } => height,
                            NodeKind::Leaf { .. } => 0,
                        };
                        let h2 = match self.nodes[child2].kind {
                            NodeKind::Internal { height, .. } => height,
                            NodeKind::Leaf { .. } => 0,
                        };
                        let height = 1 + h1.max(h2);
                        (aabb, height, node.parent, false)
                    }
                }
            };

            {
                let n = &mut self.nodes[node_h];
                n.bounding_box = new_aabb;
                if let NodeKind::Internal { ref mut height, .. } = n.kind
                    && !is_leaf
                {
                    *height = new_height;
                }
            }

            let new_root = self.balance(node_h);
            h = self.nodes[new_root].parent;
        }
    }

    fn balance(&mut self, node: Handle) -> Handle {
        let a = node;
        let a_height = match self.nodes[a].kind {
            NodeKind::Internal { height, .. } => height,
            NodeKind::Leaf { .. } => 0,
        };
        if a_height < 2 {
            return a;
        }

        let (b, c) = match self.nodes[a].kind {
            NodeKind::Internal { child1, child2, .. } => (child1, child2),
            NodeKind::Leaf { .. } => return a,
        };
        let b_height = match self.nodes[b].kind {
            NodeKind::Internal { height, .. } => height,
            NodeKind::Leaf { .. } => 0,
        };
        let c_height = match self.nodes[c].kind {
            NodeKind::Internal { height, .. } => height,
            NodeKind::Leaf { .. } => 0,
        };
        let balance = c_height - b_height;

        if balance > 1 {
            let (c1, c2) = match self.nodes[c].kind {
                NodeKind::Internal { child1, child2, .. } => (child1, child2),
                NodeKind::Leaf { .. } => return a,
            };

            // Rotate c up.
            if let NodeKind::Internal { ref mut child1, .. } = self.nodes[c].kind {
                *child1 = a;
            }
            let a_parent = self.nodes[a].parent;
            self.nodes[c].parent = a_parent;
            self.nodes[a].parent = Some(c);

            if let Some(parent) = a_parent {
                let (p_c1, _) = match self.nodes[parent].kind {
                    NodeKind::Internal { child1, child2, .. } => (child1, child2),
                    NodeKind::Leaf { .. } => unreachable!("parent must be internal"),
                };
                if p_c1 == a {
                    if let NodeKind::Internal { ref mut child1, .. } = self.nodes[parent].kind {
                        *child1 = c;
                    }
                } else if let NodeKind::Internal { ref mut child2, .. } = self.nodes[parent].kind {
                    *child2 = c;
                }
            } else {
                self.root = Some(c);
            }

            let c1_height = match self.nodes[c1].kind {
                NodeKind::Internal { height, .. } => height,
                NodeKind::Leaf { .. } => 0,
            };
            let c2_height = match self.nodes[c2].kind {
                NodeKind::Internal { height, .. } => height,
                NodeKind::Leaf { .. } => 0,
            };
            if c1_height > c2_height {
                if let NodeKind::Internal { ref mut child2, .. } = self.nodes[c].kind {
                    *child2 = c1;
                }
                if let NodeKind::Internal { ref mut child2, .. } = self.nodes[a].kind {
                    *child2 = c2;
                }
                self.nodes[c2].parent = Some(a);

                self.nodes[a].bounding_box = self.nodes[b]
                    .bounding_box
                    .union(&self.nodes[c2].bounding_box);
                self.nodes[c].bounding_box = self.nodes[a]
                    .bounding_box
                    .union(&self.nodes[c1].bounding_box);

                let b_height = match self.nodes[b].kind {
                    NodeKind::Internal { height, .. } => height,
                    NodeKind::Leaf { .. } => 0,
                };
                let c2_height = match self.nodes[c2].kind {
                    NodeKind::Internal { height, .. } => height,
                    NodeKind::Leaf { .. } => 0,
                };
                let c1_height = match self.nodes[c1].kind {
                    NodeKind::Internal { height, .. } => height,
                    NodeKind::Leaf { .. } => 0,
                };
                if let NodeKind::Internal { ref mut height, .. } = self.nodes[a].kind {
                    *height = 1 + b_height.max(c2_height);
                }
                let a_height = match self.nodes[a].kind {
                    NodeKind::Internal { height, .. } => height,
                    NodeKind::Leaf { .. } => 0,
                };
                if let NodeKind::Internal { ref mut height, .. } = self.nodes[c].kind {
                    *height = 1 + a_height.max(c1_height);
                }
            } else {
                if let NodeKind::Internal { ref mut child2, .. } = self.nodes[c].kind {
                    *child2 = c2;
                }
                if let NodeKind::Internal { ref mut child2, .. } = self.nodes[a].kind {
                    *child2 = c1;
                }
                self.nodes[c1].parent = Some(a);

                self.nodes[a].bounding_box = self.nodes[b]
                    .bounding_box
                    .union(&self.nodes[c1].bounding_box);
                self.nodes[c].bounding_box = self.nodes[a]
                    .bounding_box
                    .union(&self.nodes[c2].bounding_box);

                let b_height = match self.nodes[b].kind {
                    NodeKind::Internal { height, .. } => height,
                    NodeKind::Leaf { .. } => 0,
                };
                let c1_height = match self.nodes[c1].kind {
                    NodeKind::Internal { height, .. } => height,
                    NodeKind::Leaf { .. } => 0,
                };
                let c2_height = match self.nodes[c2].kind {
                    NodeKind::Internal { height, .. } => height,
                    NodeKind::Leaf { .. } => 0,
                };
                if let NodeKind::Internal { ref mut height, .. } = self.nodes[a].kind {
                    *height = 1 + b_height.max(c1_height);
                }
                let a_height = match self.nodes[a].kind {
                    NodeKind::Internal { height, .. } => height,
                    NodeKind::Leaf { .. } => 0,
                };
                if let NodeKind::Internal { ref mut height, .. } = self.nodes[c].kind {
                    *height = 1 + a_height.max(c2_height);
                }
            }

            return c;
        }

        if balance < -1 {
            let (b1, b2) = match self.nodes[b].kind {
                NodeKind::Internal { child1, child2, .. } => (child1, child2),
                NodeKind::Leaf { .. } => return a,
            };

            // Rotate b up.
            if let NodeKind::Internal { ref mut child1, .. } = self.nodes[b].kind {
                *child1 = a;
            }
            let a_parent = self.nodes[a].parent;
            self.nodes[b].parent = a_parent;
            self.nodes[a].parent = Some(b);

            if let Some(parent) = a_parent {
                let (p_c1, _) = match self.nodes[parent].kind {
                    NodeKind::Internal { child1, child2, .. } => (child1, child2),
                    NodeKind::Leaf { .. } => unreachable!("parent must be internal"),
                };
                if p_c1 == a {
                    if let NodeKind::Internal { ref mut child1, .. } = self.nodes[parent].kind {
                        *child1 = b;
                    }
                } else if let NodeKind::Internal { ref mut child2, .. } = self.nodes[parent].kind {
                    *child2 = b;
                }
            } else {
                self.root = Some(b);
            }

            let b1_height = match self.nodes[b1].kind {
                NodeKind::Internal { height, .. } => height,
                NodeKind::Leaf { .. } => 0,
            };
            let b2_height = match self.nodes[b2].kind {
                NodeKind::Internal { height, .. } => height,
                NodeKind::Leaf { .. } => 0,
            };
            if b1_height > b2_height {
                if let NodeKind::Internal { ref mut child2, .. } = self.nodes[b].kind {
                    *child2 = b1;
                }
                if let NodeKind::Internal { ref mut child1, .. } = self.nodes[a].kind {
                    *child1 = b2;
                }
                self.nodes[b2].parent = Some(a);

                self.nodes[a].bounding_box = self.nodes[c]
                    .bounding_box
                    .union(&self.nodes[b2].bounding_box);
                self.nodes[b].bounding_box = self.nodes[a]
                    .bounding_box
                    .union(&self.nodes[b1].bounding_box);

                let c_height = match self.nodes[c].kind {
                    NodeKind::Internal { height, .. } => height,
                    NodeKind::Leaf { .. } => 0,
                };
                let b2_height = match self.nodes[b2].kind {
                    NodeKind::Internal { height, .. } => height,
                    NodeKind::Leaf { .. } => 0,
                };
                let b1_height = match self.nodes[b1].kind {
                    NodeKind::Internal { height, .. } => height,
                    NodeKind::Leaf { .. } => 0,
                };
                if let NodeKind::Internal { ref mut height, .. } = self.nodes[a].kind {
                    *height = 1 + c_height.max(b2_height);
                }
                let a_height = match self.nodes[a].kind {
                    NodeKind::Internal { height, .. } => height,
                    NodeKind::Leaf { .. } => 0,
                };
                if let NodeKind::Internal { ref mut height, .. } = self.nodes[b].kind {
                    *height = 1 + a_height.max(b1_height);
                }
            } else {
                if let NodeKind::Internal { ref mut child2, .. } = self.nodes[b].kind {
                    *child2 = b2;
                }
                if let NodeKind::Internal { ref mut child1, .. } = self.nodes[a].kind {
                    *child1 = b1;
                }
                self.nodes[b1].parent = Some(a);

                self.nodes[a].bounding_box = self.nodes[c]
                    .bounding_box
                    .union(&self.nodes[b1].bounding_box);
                self.nodes[b].bounding_box = self.nodes[a]
                    .bounding_box
                    .union(&self.nodes[b2].bounding_box);

                let c_height = match self.nodes[c].kind {
                    NodeKind::Internal { height, .. } => height,
                    NodeKind::Leaf { .. } => 0,
                };
                let b1_height = match self.nodes[b1].kind {
                    NodeKind::Internal { height, .. } => height,
                    NodeKind::Leaf { .. } => 0,
                };
                let b2_height = match self.nodes[b2].kind {
                    NodeKind::Internal { height, .. } => height,
                    NodeKind::Leaf { .. } => 0,
                };
                if let NodeKind::Internal { ref mut height, .. } = self.nodes[a].kind {
                    *height = 1 + c_height.max(b1_height);
                }
                let a_height = match self.nodes[a].kind {
                    NodeKind::Internal { height, .. } => height,
                    NodeKind::Leaf { .. } => 0,
                };
                if let NodeKind::Internal { ref mut height, .. } = self.nodes[b].kind {
                    *height = 1 + a_height.max(b2_height);
                }
            }

            return b;
        }

        a
    }
}

impl Default for DynamicBvh {
    fn default() -> Self {
        Self {
            nodes: Arena::new(),
            root: None,
            fat_margin: 0.0,
            velocity_inflate: 0.0,
        }
    }
}
