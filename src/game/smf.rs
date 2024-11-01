#![allow(dead_code)]

use byteorder::{LittleEndian, ReadBytesExt};
use glam::{quat, vec3, Quat, Vec2, Vec3};

fn smf_version(s: &str) -> u32 {
    if s.starts_with("SMF V1.0") {
        return 1;
    }
    if s.starts_with("SMF V1.1") {
        return 2;
    }
    0
}

#[derive(Clone, Debug)]
pub struct Scene {
    pub name: String,
    pub scale: Vec3,
    pub nodes: Vec<Node>,
}

impl Scene {
    pub fn read<R>(r: &mut R) -> std::io::Result<Self>
    where
        R: std::io::Read + std::io::Seek,
    {
        skip_sinister_header(r)?;

        let version_string = read_fixed_string(r, 16);
        let smf_version = smf_version(&version_string);
        if smf_version == 0 {
            panic!("Invalid smf file version.");
        }

        let name = read_fixed_string(r, 128);

        let mut scale = Vec3::ZERO;
        scale.x = r.read_f32::<LittleEndian>()?;
        scale.y = r.read_f32::<LittleEndian>()?;
        scale.z = r.read_f32::<LittleEndian>()?;

        let _ = r.read_f32::<LittleEndian>()?; // usually == 1.0
        let _ = r.read_u32::<LittleEndian>()?; // usually == 1

        let node_count = r.read_u32::<LittleEndian>()?;

        let mut nodes = Vec::with_capacity(node_count as usize);
        for _ in 0..node_count {
            nodes.push(Node::read(r, smf_version)?);
        }

        Ok(Scene { name, scale, nodes })
    }
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub name: String,
    pub texture_name: String,
    pub vertices: Vec<Vertex>,
    pub faces: Vec<Face>,
}

#[derive(Clone, Debug)]
pub struct Face {
    pub index: u32,
    pub indices: [u32; 3],
}

impl Face {
    fn read<R>(r: &mut R) -> Self
    where
        R: std::io::Read,
    {
        let index = r.read_u32::<LittleEndian>().unwrap();
        let i0 = r.read_u32::<LittleEndian>().unwrap();
        let i1 = r.read_u32::<LittleEndian>().unwrap();
        let i2 = r.read_u32::<LittleEndian>().unwrap();

        Face {
            index,
            indices: [i0, i1, i2],
        }
    }
}

impl Mesh {
    fn read<R>(r: &mut R) -> std::io::Result<Self>
    where
        R: std::io::Read,
    {
        let name = read_fixed_string(r, 128);
        let texture_name = read_fixed_string(r, 128);

        let vertex_count = r.read_u32::<LittleEndian>().unwrap();
        let face_count = r.read_u32::<LittleEndian>().unwrap();

        let mut vertices = Vec::with_capacity(vertex_count as usize);
        for _ in 0..vertex_count {
            vertices.push(Vertex::read(r)?);
        }

        let mut faces = Vec::with_capacity(face_count as usize);
        for _ in 0..face_count {
            faces.push(Face::read(r));
        }

        Ok(Mesh {
            name,
            texture_name,
            vertices,
            faces,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Vertex {
    pub index: u32,
    pub position: Vec3,
    pub tex_coord: Vec2,
    pub normal: Vec3,
}

impl Vertex {
    fn read<R>(r: &mut R) -> std::io::Result<Self>
    where
        R: std::io::Read,
    {
        let index = r.read_u32::<LittleEndian>()?;

        let x = r.read_f32::<LittleEndian>()?;
        let y = r.read_f32::<LittleEndian>()?;
        let z = r.read_f32::<LittleEndian>()?;
        let position = vec3(x, y, z);

        let _ = r.read_i32::<LittleEndian>()?; // usually == -1
        let _ = r.read_i32::<LittleEndian>()?; // usually == 0.0

        let mut tex_coord = Vec2::ZERO;
        tex_coord.x = r.read_f32::<LittleEndian>()?;
        tex_coord.y = r.read_f32::<LittleEndian>()?;

        let x = r.read_f32::<LittleEndian>()?;
        let y = r.read_f32::<LittleEndian>()?;
        let z = r.read_f32::<LittleEndian>()?;
        let normal = vec3(x, y, z);

        Ok(Vertex {
            index,
            position,
            tex_coord,
            normal,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CollisionBox {
    pub max: Vec3,
    pub min: Vec3,
    pub u0: f32,
}

impl CollisionBox {
    fn read(r: &mut impl std::io::Read) -> Self {
        let mut max = Vec3::ZERO;
        max.x = r.read_f32::<LittleEndian>().unwrap();
        max.y = r.read_f32::<LittleEndian>().unwrap();
        max.z = r.read_f32::<LittleEndian>().unwrap();
        let mut min = Vec3::ZERO;
        min.x = r.read_f32::<LittleEndian>().unwrap();
        min.y = r.read_f32::<LittleEndian>().unwrap();
        min.z = r.read_f32::<LittleEndian>().unwrap();
        let u0 = r.read_f32::<LittleEndian>().unwrap();

        CollisionBox { max, min, u0 }
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub parent_name: String,
    pub bone_index: u32,
    pub position: Vec3,
    pub rotation: Quat,
    pub meshes: Vec<Mesh>,
    pub collision_boxes: Vec<CollisionBox>,
}

impl Node {
    fn read(r: &mut impl std::io::Read, smf_version: u32) -> std::io::Result<Self> {
        let name = read_fixed_string(r, 128);
        let parent_name = read_fixed_string(r, 128);

        let bone_index = r.read_u32::<LittleEndian>()?; // usually == 0.0

        let x = r.read_f32::<LittleEndian>()?;
        let y = r.read_f32::<LittleEndian>()?;
        let z = r.read_f32::<LittleEndian>()?;
        let position = vec3(x, y, z);

        // TODO: These components might not be in the correct order.
        let x = r.read_f32::<LittleEndian>()?;
        let y = r.read_f32::<LittleEndian>()?;
        let z = r.read_f32::<LittleEndian>()?;
        let w = r.read_f32::<LittleEndian>()?;
        let rotation = quat(x, y, z, w);

        let mesh_count = r.read_u32::<LittleEndian>()?;
        let collision_box_count = r.read_u32::<LittleEndian>()?;

        if smf_version > 1 {
            let _ = r.read_u32::<LittleEndian>()?;
        }

        let mut meshes = Vec::with_capacity(mesh_count as usize);
        for _ in 0..mesh_count {
            meshes.push(Mesh::read(r)?);
        }

        let mut collision_boxes = Vec::with_capacity(collision_box_count as usize);
        for _ in 0..collision_box_count {
            collision_boxes.push(CollisionBox::read(r));
        }

        Ok(Node {
            name,
            parent_name,
            bone_index,
            position,
            rotation,
            meshes,
            collision_boxes,
        })
    }
}

pub fn read_fixed_string(r: &mut impl std::io::Read, len: usize) -> String {
    let mut s = String::with_capacity(len);
    let mut len = len - 1;
    loop {
        let ch = r.read_u8().unwrap();
        if ch == 0 || len == 0 {
            break;
        }
        s.push(ch as char);
        len -= 1;
    }
    while len != 0 {
        r.read_u8().unwrap();
        len -= 1;
    }
    s
}

pub fn skip_sinister_header<R>(r: &mut R) -> std::io::Result<u64>
where
    R: std::io::Read + std::io::Seek,
{
    let header_start = r.stream_position()?;

    let mut ch = r.read_u8()?;
    let mut buf = vec![];
    loop {
        // Check the first character of the line.
        if ch != 0x2A {
            break;
        }

        // Consume the rest of the line.
        while ch != 0x0A && ch != 0x0D {
            buf.push(ch);
            ch = r.read_u8()?;
        }

        // Consume the newline characters.
        while ch == 0x0A || ch == 0x0D {
            buf.push(ch);
            ch = r.read_u8()?;
        }
    }

    // Read the ID string.
    // TODO: What is this really??!!
    // 1A FA 31 C1 | DE ED 42 13
    let _ = r.read_u32::<LittleEndian>()?;
    let _ = r.read_u32::<LittleEndian>()?;

    // We read into the data by 1 character, so reverse it.
    let header_end = r.seek(std::io::SeekFrom::Current(-1))?;

    Ok(header_end - header_start)
}
