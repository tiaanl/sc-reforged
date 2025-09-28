use std::ops::RangeInclusive;

use glam::{IVec2, UVec2, Vec2, Vec3};

use crate::game::{math::BoundingBox, scenes::world::terrain::Terrain};

/// A rectangular grid of evenly spaced vertices, each with a variable elevation.
///
/// Nodes are the vertices of the grid and cells are the rectangular areas between adjacent nodes.
pub struct HeightMap {
    /// Number of cells in each axis.
    pub size: UVec2,
    /// Number of chunks in each axis.
    pub chunk_count: UVec2,
    /// Edge length of each cell.
    pub cell_size: f32,
    /// Elevation index of each node.
    pub elevations: Vec<f32>,
    /// Data for each chunk.
    pub chunks: Vec<Chunk>,
}

impl HeightMap {
    pub fn from_pcx(
        data: Vec<u8>,
        elevation_scale: f32,
        cell_size: f32,
    ) -> Result<Self, std::io::Error> {
        let mut result = {
            let mut reader = pcx::Reader::from_mem(&data)?;

            let size = UVec2::new(reader.width() as u32, reader.height() as u32);
            if !reader.is_paletted() {
                return Err(std::io::ErrorKind::InvalidData.into());
            }

            let elevations = Self::read_elevations(size, elevation_scale, &mut reader)?;

            let chunk_count = size / Terrain::CELLS_PER_CHUNK;
            let chunks = vec![Chunk::default(); chunk_count.x as usize * chunk_count.y as usize];

            Self {
                size,
                chunk_count,
                cell_size,
                elevations,
                chunks,
            }
        };

        result.update_chunks();

        Ok(result)
    }

    /// Return the elevation for the specified node.
    #[inline]
    pub fn node_elevation(&self, node: IVec2) -> f32 {
        get_elevation(&self.elevations, node, self.size)
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

    fn read_elevations(
        size: UVec2,
        elevation_scale: f32,
        reader: &mut pcx::Reader<std::io::Cursor<&[u8]>>,
    ) -> Result<Vec<f32>, std::io::Error> {
        let mut elevations = vec![0_u8; size.x as usize * size.y as usize];
        for row in 0..size.y {
            let start = row as usize * size.x as usize;
            let end = (row as usize + 1) * size.x as usize;
            let slice = &mut elevations[start..end];
            reader.next_row_paletted(slice)?;
        }
        Ok(elevations
            .iter()
            .map(|index| ((u8::MAX - *index) as f32) * elevation_scale)
            .collect())
    }

    fn update_chunks(&mut self) {
        debug_assert!(
            self.chunks.len() == self.chunk_count.x as usize * self.chunk_count.y as usize
        );

        let cell_count = Terrain::CELLS_PER_CHUNK as i32;
        let elevations = &self.elevations;

        for chunk_y in 0..self.chunk_count.y {
            for chunk_x in 0..self.chunk_count.x {
                let chunk_index = chunk_y as usize * self.chunk_count.x as usize + chunk_x as usize;
                let node_y_start = chunk_y as i32 * cell_count;
                let node_x_start = chunk_x as i32 * cell_count;

                let chunk = &mut self.chunks[chunk_index];
                chunk.node_y_range = node_y_start..=node_y_start + cell_count;
                chunk.node_x_range = node_x_start..=node_x_start + cell_count;

                let chunk_min = Vec2::new(
                    *chunk.node_x_range.start() as f32 * self.cell_size,
                    *chunk.node_y_range.start() as f32 * self.cell_size,
                );
                let chunk_max = Vec2::new(
                    *chunk.node_x_range.end() as f32 * self.cell_size,
                    *chunk.node_y_range.end() as f32 * self.cell_size,
                );

                let mut min_z = f32::INFINITY;
                let mut max_z = f32::NEG_INFINITY;

                for node_y in chunk.node_y_range.clone() {
                    for node_x in chunk.node_x_range.clone() {
                        let elevation =
                            get_elevation(elevations, IVec2::new(node_x, node_y), self.size);
                        min_z = min_z.min(elevation);
                        max_z = max_z.max(elevation);
                    }
                }

                chunk.bounding_box.min = chunk_min.extend(min_z);
                chunk.bounding_box.max = chunk_max.extend(max_z);
            }
        }
    }
}

fn get_elevation(elevations: &[f32], coord: IVec2, size: UVec2) -> f32 {
    // The last node is clamped, which will result in a flat cell.
    let coord = coord.clamp(
        IVec2::ZERO,
        IVec2::new(size.x as i32, size.y as i32) - IVec2::ONE,
    );
    let index = coord.y as usize * size.x as usize + coord.x as usize;
    elevations[index]
}

#[derive(Clone, Debug)]
pub struct Chunk {
    pub node_x_range: RangeInclusive<i32>,
    pub node_y_range: RangeInclusive<i32>,
    pub bounding_box: BoundingBox,
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            node_x_range: 0..=0,
            node_y_range: 0..=0,
            bounding_box: Default::default(),
        }
    }
}
