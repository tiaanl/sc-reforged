use glam::{IVec2, UVec2, Vec2, Vec3};

/// A rectangular grid of evenly spaced vertices, each with a variable elevation.
///
/// Nodes are the vertices of the grid and cells are the rectangular areas between adjacent nodes.
pub struct HeightMap {
    /// X and Y sizes counted in amount of nodes (*not* cells!).
    pub size: UVec2,
    /// Edge length of each cell.
    pub cell_size: f32,
    /// Elevation index of each node.
    pub elevations: Vec<f32>,
}

impl HeightMap {
    pub fn from_pcx(
        data: Vec<u8>,
        elevation_scale: f32,
        cell_size: f32,
    ) -> Result<Self, std::io::Error> {
        let mut reader = pcx::Reader::from_mem(&data)?;

        let size = UVec2::new(reader.width() as u32, reader.height() as u32);
        if !reader.is_paletted() {
            return Err(std::io::ErrorKind::InvalidData.into());
        }

        let mut elevations = vec![0_u8; size.x as usize * size.y as usize];
        for row in 0..size.y {
            let start = row as usize * size.x as usize;
            let end = (row as usize + 1) * size.x as usize;
            let slice = &mut elevations[start..end];
            reader.next_row_paletted(slice)?;
        }

        Ok(Self {
            size,
            cell_size,
            elevations: elevations
                .iter()
                .map(|index| ((u8::MAX - *index) as f32) * elevation_scale)
                .collect(),
        })
    }

    /// Return the elevation for the specified node.
    pub fn node_elevation(&self, node: IVec2) -> f32 {
        let node = self.clamped_node_pos(node);
        let index = node.y as usize * self.size.x as usize + node.x as usize;
        self.elevations[index]
    }

    /// Return the world position for the specified node.
    pub fn node_world_position(&self, node: IVec2) -> Vec3 {
        let elevation = self.node_elevation(node);
        let x = node.x as f32 * self.cell_size;
        let y = node.y as f32 * self.cell_size;
        Vec3::new(x, y, elevation)
    }

    /// Return the world position and surface normal at the given world (x,y).
    pub fn world_position_and_normal(&self, world_pos: Vec2) -> (Vec3, Vec3) {
        let local = world_pos / self.cell_size;

        let node = IVec2::new(local.x.floor() as i32, local.y.floor() as i32);
        let t = Vec2::new(local.x - node.x as f32, local.y - node.y as f32);

        let h00 = self.node_elevation(node);
        let h10 = self.node_elevation(node + IVec2::X);
        let h01 = self.node_elevation(node + IVec2::Y);
        let h11 = self.node_elevation(node + IVec2::ONE);

        // Bilinear elevation
        let hx0 = h00 * (1.0 - t.x) + h10 * t.x;
        let hx1 = h01 * (1.0 - t.x) + h11 * t.x;
        let elevation = hx0 * (1.0 - t.y) + hx1 * t.y;

        let pos = world_pos.extend(elevation);

        // Partial derivatives of the bilinear surface.
        let dh_dx = ((h10 - h00) * (1.0 - t.y) + (h11 - h01) * t.y) / self.cell_size;
        let dh_dy = ((h01 - h00) * (1.0 - t.x) + (h11 - h10) * t.x) / self.cell_size;

        // Geometric normal is (-∂h/∂x, -∂h/∂y, 1).
        let normal = Vec3::new(-dh_dx, -dh_dy, 1.0).normalize();

        (pos, normal)
    }

    /// Clamp the node coordinates to the available elevation data.
    ///
    /// NOTE: Because we only store data for nodes and *not* cells, the last cell of the terrain
    ///       will be a flat cell, because of the clamping.
    #[inline]
    fn clamped_node_pos(&self, node: IVec2) -> IVec2 {
        IVec2::new(
            node.x.clamp(0, self.size.x as i32 - 1),
            node.y.clamp(0, self.size.y as i32 - 1),
        )
    }
}
