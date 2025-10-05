use glam::{IVec2, UVec2, Vec3, Vec4, ivec2};

pub struct NewHeightMap {
    /// Amount of nodes in the height map.
    pub size: UVec2,
    /// Size of each cell/space between each node; in cm.
    pub cell_size: f32,
    /// Each node of the height map represented as [normal, altitude].
    pub nodes: Vec<Vec4>,
}

impl NewHeightMap {
    pub fn from_iter(size: UVec2, cell_size: f32, nodes: impl Iterator<Item = f32>) -> Self {
        let mut height_map = Self {
            size,
            cell_size,
            nodes: nodes.map(|n| Vec3::ZERO.extend(n)).collect(),
        };

        height_map.recalculate_normals();

        height_map
    }

    #[inline]
    pub fn node_at(&self, coord: IVec2) -> Vec4 {
        let coord = coord.clamp(IVec2::ZERO, self.size.as_ivec2() - IVec2::ONE);
        self.nodes[coord.y as usize * self.size.x as usize + coord.x as usize]
    }

    #[inline]
    pub fn world_position_at(&self, coord: IVec2) -> Vec3 {
        let altitude = self.node_at(coord).w;

        Vec3::new(
            coord.x as f32 * self.cell_size,
            coord.y as f32 * self.cell_size,
            altitude,
        )
    }

    fn recalculate_normals(&mut self) {
        let size = self.size.as_ivec2();

        // Force normals on the edges to be straight up. This is what the game does.
        for y in 0..size.y {
            for x in 0..size.x {
                if x == 0 || y == 0 || x == size.x - 1 || y == size.y - 1 {
                    let node = &mut self.nodes[y as usize * size.x as usize + x as usize];
                    node.x = 0.0;
                    node.y = 0.0;
                    node.z = 1.0;
                    continue;
                }

                let center = self.world_position_at(ivec2(x, y));

                let [north, east, south, west] = [
                    ivec2(x, y + 1),
                    ivec2(x - 1, y),
                    ivec2(x, y - 1),
                    ivec2(x + 1, y),
                ]
                .map(|coord| (center - self.world_position_at(coord)).normalize());

                let n0 = north.cross(east).normalize();
                let n1 = east.cross(south).normalize();
                let n2 = south.cross(west).normalize();
                let n3 = west.cross(north).normalize();

                let n = (n0 + n1 + n2 + n3).normalize();

                let node = &mut self.nodes[y as usize * size.x as usize + x as usize];
                node.x = n.x;
                node.y = n.y;
                node.z = n.z;
            }
        }
    }
}
