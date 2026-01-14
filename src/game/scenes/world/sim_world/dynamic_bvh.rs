use bevy_ecs::prelude::*;
use generational_arena::{Arena, Index as Handle};
use glam::Vec3;

use crate::game::math::{BoundingBox, Containment, Frustum, RaySegment};

#[derive(Clone, Copy, Component)]
pub struct DynamicBvhHandle(Handle);

/// Node stored in the arena. Internal nodes have two children; leaf nodes have
/// an object payload.
#[derive(Clone)]
struct Node<T> {
    bounding_box: BoundingBox,

    parent: Option<Handle>,
    child1: Option<Handle>,
    child2: Option<Handle>,

    // Leaf payload. Internal nodes have None.
    object: Option<T>,

    // Used by balancing heuristics (AVL-like rotations, etc.)
    height: i32,
}

impl<T> Node<T> {
    fn is_leaf(&self) -> bool {
        self.child1.is_none() && self.child2.is_none()
    }
}

/// Dynamic BVH tree.
#[derive(Resource)]
pub struct DynamicBvh<T: Copy> {
    nodes: Arena<Node<T>>,
    root: Option<Handle>,

    // Tuning
    fat_margin: f32,       // world units expansion
    velocity_inflate: f32, // how much to expand by displacement (optional)
}

impl<T: Copy> DynamicBvh<T> {
    pub fn new(fat_margin: f32) -> Self {
        Self {
            nodes: Arena::new(),
            root: None,
            fat_margin: fat_margin.max(0.0),
            velocity_inflate: 1.0,
        }
    }

    pub fn with_velocity_inflate(mut self, k: f32) -> Self {
        self.velocity_inflate = k.max(0.0);
        self
    }

    /// Insert object with its *tight* AABB. Returns a handle you store externally.
    pub fn insert(&mut self, object: T, tight: BoundingBox) -> DynamicBvhHandle {
        let fat = self.fatten_aabb(tight, Vec3::ZERO);

        let leaf = self.nodes.insert(Node {
            bounding_box: fat,
            parent: None,
            child1: None,
            child2: None,
            object: Some(object),
            height: 0,
        });

        self.insert_leaf(leaf);

        DynamicBvhHandle(leaf)
    }

    /// Remove a previously inserted handle.
    pub fn remove(&mut self, handle: DynamicBvhHandle) -> Option<T> {
        if !self.nodes.contains(handle.0) {
            return None;
        }

        self.remove_leaf(handle.0);

        // Take the object out and remove the node from arena.
        let obj = self.nodes[handle.0].object.take();
        let _ = self.nodes.remove(handle.0);
        obj
    }

    /// Update an object's tight AABB.
    ///
    /// `displacement` is (new_center - old_center) or velocity*dt; used to inflate fat AABB so
    /// fast movers don’t thrash reinserts.
    ///
    /// Returns true if the proxy was reinserted (tree topology changed).
    pub fn update(&mut self, handle: Handle, new_tight: BoundingBox, displacement: Vec3) -> bool {
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

    /// Optional: direct access to the fat AABB for debugging/visualization.
    pub fn fat_aabb(&self, handle: Handle) -> Option<BoundingBox> {
        self.nodes.get(handle).map(|n| n.bounding_box)
    }

    // --------------------
    // Queries
    // --------------------

    pub fn query_frustum(&self, frustum: &Frustum, out: &mut Vec<T>) {
        out.clear();
        let Some(root) = self.root else {
            return;
        };

        // (node, parent_fully_inside)
        let mut stack: Vec<(Handle, bool)> = Vec::new();
        stack.push((root, false));

        while let Some((h, parent_inside)) = stack.pop() {
            let node = &self.nodes[h];

            if parent_inside {
                if node.is_leaf() {
                    if let Some(obj) = node.object {
                        out.push(obj);
                    }
                } else {
                    stack.push((node.child1.unwrap(), true));
                    stack.push((node.child2.unwrap(), true));
                }
                continue;
            }

            match frustum.vs_bounding_box(&node.bounding_box) {
                Containment::Outside => {}
                Containment::Inside => {
                    if node.is_leaf() {
                        if let Some(obj) = node.object {
                            out.push(obj);
                        }
                    } else {
                        stack.push((node.child1.unwrap(), true));
                        stack.push((node.child2.unwrap(), true));
                    }
                }
                Containment::Intersect => {
                    if node.is_leaf() {
                        // If you want extra precision you can keep tight boxes externally and test them here.
                        if let Some(obj) = node.object {
                            out.push(obj);
                        }
                    } else {
                        stack.push((node.child1.unwrap(), false));
                        stack.push((node.child2.unwrap(), false));
                    }
                }
            }
        }
    }

    pub fn query_ray_segment(&self, ray: &RaySegment, out: &mut Vec<T>) {
        out.clear();
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

            if node.is_leaf() {
                if let Some(obj) = node.object {
                    out.push(obj);
                }
            } else {
                stack.push(node.child1.unwrap());
                stack.push(node.child2.unwrap());
            }
        }
    }

    // --------------------
    // Internals
    // --------------------

    fn fatten_aabb(&self, tight: BoundingBox, displacement: Vec3) -> BoundingBox {
        // You’ll implement these ops on BoundingBox in your math module:
        // - expand_by_scalar(margin)
        // - expand_by_vec3(abs(displacement) * k)
        // - or expand_to_include point/box
        let mut fat = tight;
        fat.expand(Vec3::splat(self.fat_margin));

        let disp = displacement.abs() * self.velocity_inflate;
        fat.expand(disp);

        fat
    }

    fn insert_leaf(&mut self, leaf: Handle) {
        // Empty tree.
        if self.root.is_none() {
            self.root = Some(leaf);
            self.nodes[leaf].parent = None;
            return;
        }

        // ----------------------------
        // Helpers (local AABB ops)
        // ----------------------------
        #[inline]
        fn aabb_union(a: &BoundingBox, b: &BoundingBox) -> BoundingBox {
            BoundingBox {
                min: a.min.min(b.min),
                max: a.max.max(b.max),
            }
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
        while !self.nodes[index].is_leaf() {
            let c1 = self.nodes[index].child1.unwrap();
            let c2 = self.nodes[index].child2.unwrap();

            let cost1 = {
                let u = aabb_union(&leaf_aabb, &self.nodes[c1].bounding_box);
                surface_area(&u) - surface_area(&self.nodes[c1].bounding_box)
            };

            let cost2 = {
                let u = aabb_union(&leaf_aabb, &self.nodes[c2].bounding_box);
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

        let new_parent_aabb = aabb_union(&leaf_aabb, &self.nodes[sibling].bounding_box);

        let new_parent = self.nodes.insert(Node {
            bounding_box: new_parent_aabb,
            parent: old_parent,
            child1: Some(sibling),
            child2: Some(leaf),
            object: None,
            height: self.nodes[sibling].height.max(self.nodes[leaf].height) + 1,
        });

        // Fix parent pointers on children.
        self.nodes[sibling].parent = Some(new_parent);
        self.nodes[leaf].parent = Some(new_parent);

        // ----------------------------
        // 3) Hook new parent into the old parent (or become root)
        // ----------------------------
        if let Some(p) = old_parent {
            let (p_c1, p_c2) = {
                let pnode = &self.nodes[p];
                (pnode.child1.unwrap(), pnode.child2.unwrap())
            };

            if p_c1 == sibling {
                self.nodes[p].child1 = Some(new_parent);
            } else {
                debug_assert!(p_c2 == sibling);
                self.nodes[p].child2 = Some(new_parent);
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
        let (c1, c2) = {
            let p = &self.nodes[parent];
            (
                p.child1.expect("internal node child1"),
                p.child2.expect("internal node child2"),
            )
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
            let (gp_c1, gp_c2) = {
                let g = &self.nodes[gp];
                (
                    g.child1.expect("internal node child1"),
                    g.child2.expect("internal node child2"),
                )
            };

            if gp_c1 == parent {
                self.nodes[gp].child1 = Some(sibling);
            } else {
                debug_assert!(gp_c2 == parent);
                self.nodes[gp].child2 = Some(sibling);
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
            let (new_aabb, new_height, parent) = {
                let node = &self.nodes[node_h];

                if node.is_leaf() {
                    (node.bounding_box, 0, node.parent)
                } else {
                    let c1 = node.child1.unwrap();
                    let c2 = node.child2.unwrap();
                    let aabb = self.nodes[c1]
                        .bounding_box
                        .union(&self.nodes[c2].bounding_box);
                    let height = 1 + self.nodes[c1].height.max(self.nodes[c2].height);
                    (aabb, height, node.parent)
                }
            };

            {
                let n = &mut self.nodes[node_h];
                n.bounding_box = new_aabb;
                n.height = new_height;
            }

            // Optionally rebalance at node_h here:
            // self.balance(node_h);

            h = parent;
        }
    }

    fn balance(&mut self, node: Handle) -> Handle {
        // Perform rotations to reduce height / improve SAH.
        // Box2D uses height-based balancing with rotations.
        let _ = node;
        todo!("balance: rotate nodes; return new subtree root");
    }
}
