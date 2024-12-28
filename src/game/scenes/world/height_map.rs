use glam::{IVec2, UVec2, Vec3};

pub struct HeightMap {
    pub edge_size: f32,
    pub elevation_base: f32,

    pub size: UVec2,
    pub elevations: Vec<u32>,
}

impl HeightMap {
    pub fn from_reader<R>(
        edge_size: f32,
        elevation_base: f32,
        reader: &mut R,
    ) -> Result<Self, std::io::Error>
    where
        R: std::io::Read,
    {
        let data = pcx::load_height_map(reader)?;

        Ok(Self {
            edge_size,
            elevation_base,
            size: UVec2 {
                x: data.width,
                y: data.height,
            },
            elevations: data.data,
        })
    }

    /// Return the world position of the specified height map coordinate.
    ///
    /// NOTE: Coordinates outside the height map area will return the value of the nearest edge
    ///       coordinate. This will cause all the far edges of the heightmap to have a single flat
    ///       cell on the map edge. This is to replicate behavious on the original.
    pub fn position_for_vertex(&self, pos: IVec2) -> Vec3 {
        let IVec2 { x, y } = pos;

        // Clamp to the available data.
        let index = y.max(0).min(self.size.y as i32 - 1) as usize * self.size.x as usize
            + x.max(0).min(self.size.x as i32 - 1) as usize;

        let elevation = (self.elevations[index] as usize & 0xFF) as f32 * self.elevation_base;
        Vec3::new(
            x as f32 * self.edge_size,
            y as f32 * self.edge_size,
            elevation,
        )
    }
}

mod pcx {
    use byteorder::{LittleEndian as LE, ReadBytesExt};

    pub struct Data {
        pub width: u32,
        pub height: u32,
        pub data: Vec<u32>,
    }

    #[repr(C)]
    struct PcxHeader {
        // always 0x0A
        manufacturer: u8,

        // 0 = v2.5
        // 2 = v2.8 with palette
        // 3 = v2.8 without palette
        // 4 = Paintbrush for Windows
        // 5 = v3.0 or higher
        version: u8,

        // 0 = uncompressed image (not officially allowed)
        // 1 = PCX run length encoding
        // should be 0x01
        encoding: u8,

        // number of bits per pixel in each entry of the color planes
        bits_per_plane: u8,

        x_min: u16,
        y_min: u16,
        x_max: u16,
        y_max: u16,

        horizontal_dpi: u16,
        vertical_dpi: u16,

        // palette for 16 colors or less, in three-byte RGB entries.
        palette: [u8; 48],

        // should be set to 0.
        reserved: u8,

        // Number of color planes. Multiply by bits_per_pixel to fet the actual color depth.
        color_planes: u8,

        // number of bytes to read for a single plane's scanline.
        bytes_per_plane_line: u16,

        // 1 = color/bw
        // 2 = grayscale
        palette_info: u16,

        // deals with scrolling, best to just ignore.
        horizontal_screen_size: u16,
        vertical_screen_size: u16,

        padding: [u8; 54],
    }

    fn read_pcx_header<R>(reader: &mut R) -> std::io::Result<PcxHeader>
    where
        R: std::io::Read,
    {
        let mut header: PcxHeader = unsafe {
            // We will overwrite all the fields, so we can leave them as garbage.
            std::mem::MaybeUninit::uninit().assume_init_read()
        };

        header.manufacturer = reader.read_u8()?;
        header.version = reader.read_u8()?;
        header.encoding = reader.read_u8()?;
        header.bits_per_plane = reader.read_u8()?;
        header.x_min = reader.read_u16::<LE>()?;
        header.y_min = reader.read_u16::<LE>()?;
        header.x_max = reader.read_u16::<LE>()?;
        header.y_max = reader.read_u16::<LE>()?;
        header.horizontal_dpi = reader.read_u16::<LE>()?;
        header.vertical_dpi = reader.read_u16::<LE>()?;
        reader.read_exact(&mut header.palette)?;
        header.reserved = reader.read_u8()?;
        header.color_planes = reader.read_u8()?;
        header.bytes_per_plane_line = reader.read_u16::<LE>()?;
        header.palette_info = reader.read_u16::<LE>()?;
        header.horizontal_screen_size = reader.read_u16::<LE>()?;
        header.vertical_screen_size = reader.read_u16::<LE>()?;
        reader.read_exact(&mut header.padding)?;

        Ok(header)
    }

    pub fn load_height_map<R>(reader: &mut R) -> std::io::Result<Data>
    where
        R: std::io::Read,
    {
        let header = read_pcx_header(reader)?;

        if header.manufacturer != 0x0A || header.version != 0x05 {
            panic!("Incorrect/invalid PCX header.");
        }

        let width = header.bytes_per_plane_line as u32;
        let height = (header.y_max - header.y_min + 1) as u32;
        let area = width * height;

        tracing::info!("width: {}, height: {}", width as u32, height as u32);

        let mut data: Vec<u32> = Vec::with_capacity(area as usize);

        macro_rules! b {
            ($b:expr) => {{
                0x1FF00_u32 | (0xFF - $b) as u32
            }};
        }

        let mut i = 0_usize;
        while i < area as usize {
            let byte = reader.read_u8()?;
            if (byte & 0xC0) != 0xC0 {
                data.push(b!(byte));
                i += 1;
            } else {
                let count = (byte & 0x3F) as usize;
                let new_byte = reader.read_u8()?;
                data.extend(std::iter::repeat(b!(new_byte)).take(count));
                i += count;
            }
        }

        assert_eq!(data.len(), area as usize);

        Ok(Data {
            width,
            height,
            data,
        })
    }
}
