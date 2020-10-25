use std::collections::HashMap;
use std::io::Write;
use std::fs::File;
use std::fmt::{Debug, Formatter};
use vulkano;

pub type Vec2 = [f32; 2];
pub type Vec3 = [f32; 3];

const FILE_VERSION_NUMBER: u32 = 1;

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct Vertex {
    position: Vec3,
    normal: Vec3,
    tex_coord: Vec2
}

vulkano::impl_vertex!(Vertex, position, normal, tex_coord);

impl Vertex {
    pub fn new_empty() -> Vertex {
        Vertex {
            position: [0.0, 0.0, 0.0],
            normal: [0.0, 0.0, 0.0],
            tex_coord: [0.0, 0.0]
        }
    }

    pub fn from_components(position: &Vec3, normal: &Vec3, tex_coord: &Vec2) -> Vertex {
        Vertex {
            position: [position[0], position[1], position[2]],
            normal: [normal[0], normal[1], normal[2]],
            tex_coord: [tex_coord[0], tex_coord[1]]
        }
    }

    fn positions_normals(&self) -> [f32; 6] {
        [ self.position[0], self.position[1], self.position[2], self.normal[0], self.normal[1], self.normal[2] ]
    }

    fn positions_tex_coords(&self) -> [f32; 5] {
        [ self.position[0], self.position[1], self.position[2], self.tex_coord[0], self.tex_coord[1] ]
    }

    fn positions(&self) -> [f32; 3] {
        [ self.position[0], self.position[1], self.position[2] ]
    }
}

pub struct RawModelData {
    raw_positions: Vec<Vec3>,
    raw_tex_coords: Vec<Vec2>,
    raw_normals: Vec<Vec3>
}

impl RawModelData {
    pub fn new() -> RawModelData {
        RawModelData {
            raw_positions: vec![],
            raw_tex_coords: vec![],
            raw_normals: vec![]
        }
    }

    pub fn push_position(&mut self, position: Vec3) {
        self.raw_positions.push(position);
    }

    pub fn push_normal(&mut self, normal: Vec3) {
        self.raw_normals.push(normal);
    }

    pub fn push_tex_coord(&mut self, tex_coord: Vec2) {
        self.raw_tex_coords.push(tex_coord);
    }

    pub fn get_raw_position(&self, index: u16) -> Option<&Vec3> {
        self.raw_positions.get(index as usize)
    }

    pub fn get_raw_normal(&self, index: u16) -> Option<&Vec3> {
        self.raw_normals.get(index as usize)
    }

    pub fn get_raw_tex_coord(&self, index: u16) -> Option<&Vec2> {
        self.raw_tex_coords.get(index as usize)
    }
}

pub struct Model {
    name: String,
    pub interleaved_vertices: Vec<Vertex>,
    pub face_indices: Vec<u16>,
    index_map: HashMap<u64, u16>
}

impl Model {
    pub fn new(model_name: String) -> Model {
        Model {
            name: model_name,
            interleaved_vertices: vec![],
            face_indices: vec![],
            index_map: HashMap::new()
        }
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_index(&mut self, index_position: u64, index_normal: u64, index_tex_coord: u64, vertex: Vertex) -> u16 {
        let identifier: u64 = index_tex_coord + (index_normal << 16) + (index_position << 32);
        match self.index_map.get(&identifier) {
            Some(position) => {
                *position
            },
            None => {
                let new_index = self.interleaved_vertices.len() as u16;
                self.index_map.insert(identifier, new_index);
                self.interleaved_vertices.push (vertex);
                new_index
            }
        }
    }

    pub fn add_face(&mut self, indices: [u16; 3]) {
        self.face_indices.push(indices[0]);
        self.face_indices.push(indices[1]);
        self.face_indices.push(indices[2]);
    }

    pub unsafe fn write_data_to_file(&self, file: &mut File, include_normals: bool, include_tex_coords: bool) -> std::io::Result<()> {
        file.write_all(&FILE_VERSION_NUMBER.to_ne_bytes())?;
        file.write_all(&(include_normals as i32).to_ne_bytes())?;
        file.write_all(&(include_tex_coords as i32).to_ne_bytes())?;

        let vertex_count = self.interleaved_vertices.len() as u32;
        file.write_all(&vertex_count.to_ne_bytes())?;
        match (include_normals, include_tex_coords) {
            (true, true) => {
                for vertex in self.interleaved_vertices.iter() {
                    file.write_all(std::mem::transmute::<&Vertex, &[u8; 32]>(vertex))?;
                }
            },
            (true, false) => {
                for vertex in self.interleaved_vertices.iter() {
                    file.write_all(std::mem::transmute::<&[f32; 6], &[u8; 24]>(&vertex.positions_normals()))?;
                }
            }
            (false, true) => {
                for vertex in self.interleaved_vertices.iter() {
                    file.write_all(std::mem::transmute::<&[f32; 5], &[u8; 20]>(&vertex.positions_tex_coords()))?;
                }
            }
            _ => {
                for vertex in self.interleaved_vertices.iter() {
                    file.write_all(std::mem::transmute::<&[f32; 3], &[u8; 12]>(&vertex.positions()))?;
                }
            }
        }

        let index_count = self.face_indices.len() as u32;
        file.write_all(&index_count.to_ne_bytes())?;
        for face_index_set in self.face_indices.iter() {
            file.write_all(std::mem::transmute::<&u16, &[u8; 6]>(face_index_set))?;
        }

        Ok(())
    }

    pub unsafe fn from_bytes(bytes: &Vec<u8>) -> Model {
        let stream = bytes.as_slice();
        let version_ptr = stream.as_ptr();
        let normals_ptr = stream[4..8].as_ptr();
        let tex_coords_ptr = stream[8..12].as_ptr();

        let version_number = *std::mem::transmute::<*const u8, *const u32>(version_ptr);
        let include_normals = *std::mem::transmute::<*const u8, *const i32>(normals_ptr) != 0;
        let include_tex_coords = *std::mem::transmute::<*const u8, *const i32>(tex_coords_ptr) != 0;
        if version_number != FILE_VERSION_NUMBER {
            panic!("Bad file version: expected {} but was {}", FILE_VERSION_NUMBER, version_number);
        }
        if !include_normals {
            panic!("Excluding normals is not currently supported");
        }
        if !include_tex_coords {
            panic!("Excluding texture coordinates is not currently supported");
        }

        let vertex_count_ptr = stream[12..16].as_ptr();
        let vertex_count = *std::mem::transmute::<*const u8, *const u32>(vertex_count_ptr);
        let mut interleaved_vertices: Vec<Vertex> = vec![Vertex::new_empty(); vertex_count as usize];
        let vertex_data_ptr = stream[16..(16 + vertex_count as usize * 8 * 4)].as_ptr();
        let vertex_ptr = std::mem::transmute::<*const u8, *const Vertex>(vertex_data_ptr);
        let vertex_slice = std::slice::from_raw_parts(vertex_ptr, vertex_count as usize);
        interleaved_vertices.copy_from_slice(vertex_slice);

        let face_count_offset = (16 + vertex_count * 8 * 4) as usize;
        let face_count_ptr = stream[face_count_offset..(face_count_offset + 4)].as_ptr();
        let face_count = *std::mem::transmute::<*const u8, *const u32>(face_count_ptr);
        let mut face_indices: Vec<u16> = vec![0u16; (face_count * 3) as usize];
        let face_data_ptr = stream[(face_count_offset + 4)..].as_ptr();
        let face_ptr = std::mem::transmute::<*const u8, *const u16>(face_data_ptr);
        let face_slice = std::slice::from_raw_parts(face_ptr, (face_count * 3) as usize);
        face_indices.copy_from_slice(face_slice);

        Model {
            name: String::from(""),
            interleaved_vertices,
            face_indices,
            index_map: HashMap::new()
        }
    }
}

impl Debug for Model {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Model")
            .field("name", &self.name)
            .field("vertices", &self.interleaved_vertices.len())
            .field("faces", &self.face_indices.len())
            .finish()
    }
}
