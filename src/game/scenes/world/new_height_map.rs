use glam::{UVec2, Vec3, Vec4, uvec2};

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
    pub fn node_at(&self, coord: UVec2) -> Vec4 {
        let coord = coord.clamp(UVec2::ZERO, self.size - UVec2::ONE);
        self.nodes[coord.y as usize * self.size.x as usize + coord.x as usize]
    }

    #[inline]
    pub fn world_position_at(&self, coord: UVec2) -> Vec3 {
        let altitude = self.node_at(coord).w;

        Vec3::new(
            coord.x as f32 * self.cell_size,
            coord.y as f32 * self.cell_size,
            altitude,
        )
    }

    fn recalculate_normals(&mut self) {
        for y in 1..self.size.y - 1 {
            for x in 1..self.size.x - 1 {
                let center = self.world_position_at(uvec2(x, y));

                let [north, east, south, west] = [
                    UVec2::new(x, y + 1),
                    UVec2::new(x - 1, y),
                    UVec2::new(x, y - 1),
                    UVec2::new(x + 1, y),
                ]
                .map(|coord| (center - self.world_position_at(coord)).normalize());

                let n0 = north.cross(east).normalize();
                let n1 = east.cross(south).normalize();
                let n2 = south.cross(west).normalize();
                let n3 = west.cross(north).normalize();

                let n = (n0 + n1 + n2 + n3).normalize();

                let node = &mut self.nodes[y as usize * self.size.x as usize + x as usize];
                node.x = n.x;
                node.y = n.y;
                node.z = n.z;
            }
        }
    }
}
