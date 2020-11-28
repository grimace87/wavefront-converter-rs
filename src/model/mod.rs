use std::collections::HashMap;
use std::io::Write;
use std::fs::File;
use std::fmt::{Debug, Formatter};

use crate::modelfactory::FILE_VERSION_NUMBER;

pub type Vec2 = [f32; 2];
pub type Vec3 = [f32; 3];

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub tex_coord: Vec2
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
}

pub struct RawModelData {
    raw_positions: Vec<Vec3>,
    raw_tex_coords: Vec<Vec2>,
    raw_normals: Vec<Vec3>
}

impl RawModelData {
    fn new() -> RawModelData {
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

impl Default for RawModelData {
    fn default() -> Self {
        Self::new()
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

    /// # Safety
    /// Should be safe to use - current self should have well-formed vertex data Vecs
    pub unsafe fn write_data_to_file(&self, file: &mut File) -> std::io::Result<()> {
        file.write_all(&FILE_VERSION_NUMBER.to_ne_bytes())?;

        let vertex_count = self.interleaved_vertices.len() as u32;
        file.write_all(&vertex_count.to_ne_bytes())?;
        for vertex in self.interleaved_vertices.iter() {
            file.write_all(&*(vertex as *const Vertex as *const [u8; 32]))?;
        }

        let face_count = (self.face_indices.len() / 3) as u32;
        file.write_all(&face_count.to_ne_bytes())?;
        for face_index_set in self.face_indices.iter() {
            file.write_all(&*(face_index_set as *const u16 as *const [u8; 2]))?;
        }

        Ok(())
    }

    /// # Safety
    /// Should be safe if processing files generated with the same version of this tool
    pub unsafe fn from_bytes(bytes: &[u8]) -> Model {
        let version_ptr = bytes.as_ptr();

        let version_number = *(version_ptr as *const u32);
        if version_number != FILE_VERSION_NUMBER {
            panic!("Bad file version: expected {} but was {}", FILE_VERSION_NUMBER, version_number);
        }

        let vertex_count_ptr = bytes[4..8].as_ptr();
        let vertex_count = *(vertex_count_ptr as *const u32);
        let mut interleaved_vertices: Vec<Vertex> = vec![Vertex::new_empty(); vertex_count as usize];
        let vertex_data_ptr = bytes[8..(8 + vertex_count as usize * 8 * 4)].as_ptr();
        let vertex_ptr = vertex_data_ptr as *const Vertex;
        let vertex_slice = std::slice::from_raw_parts(vertex_ptr, vertex_count as usize);
        interleaved_vertices.copy_from_slice(vertex_slice);

        let face_count_offset = (8 + vertex_count * 8 * 4) as usize;
        let face_count_ptr = bytes[face_count_offset..(face_count_offset + 4)].as_ptr();
        let face_count = *(face_count_ptr as *const u32);
        let mut face_indices: Vec<u16> = vec![0u16; (face_count * 3) as usize];
        let face_data_ptr = bytes[(face_count_offset + 4)..].as_ptr();
        let face_ptr = face_data_ptr as *const u16;
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
