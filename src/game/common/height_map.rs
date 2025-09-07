use glam::{IVec2, UVec2, Vec3};

/// A rectangular grid of evenly spaced vertices, each with a variable elevation.
///
/// Nodes are the vertices of the grid and cells are the rectangular areas between adjacent nodes.
pub struct HeightMap {
    /// X and Y sizes counted in amount of nodes (*not* cells!).
    pub size: UVec2,
    /// Elevation index of each node.
    pub elevations: Vec<u8>,
}

impl HeightMap {
    pub fn from_pcx(data: Vec<u8>) -> Result<Self, std::io::Error> {
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

        elevations.iter_mut().for_each(|i| {
            *i = u8::MAX - *i;
        });

        Ok(Self { size, elevations })
    }

    pub fn elevation_at(&self, node: IVec2) -> u8 {
        let x = node.x.clamp(0, self.size.x as i32 - 1);
        let y = node.y.clamp(0, self.size.y as i32 - 1);

        let index = y as usize * self.size.x as usize + x as usize;

        self.elevations[index]
    }

    /// Returns the world position of the specified node.
    ///
    /// NOTE: Coordinates outside the height map are clamped to the nearest edge, creating a flat
    /// outer edge to replicate the original behavior.
    pub fn position_for_vertex(
        &self,
        pos: IVec2,
        nominal_edge_size: f32,
        elevation_base: f32,
    ) -> Vec3 {
        let elevation = self.elevation_at(pos);

        Vec3::new(
            pos.x as f32 * nominal_edge_size,
            pos.y as f32 * nominal_edge_size,
            elevation as f32 * elevation_base,
        )
    }
}
