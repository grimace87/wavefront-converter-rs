use std::fs;
use std::path::{Path, PathBuf};
use std::str::Lines;

use crate::model::{RawModelData, Model, Vertex};
use std::fs::File;

const KEY_OBJECT: &str = "o";
const KEY_VERTEX: &str = "v";
const KEY_NORMAL: &str = "vn";
const KEY_TEX_COORD: &str = "vt";
const KEY_FACE: &str = "f";

struct IndexSet {
    position_index: u16,
    normal_index: u16,
    tex_coord_index: u16
}

pub struct ModelFactory {
    source_file_path: PathBuf,
    raw_model_data: RawModelData,
    models: Vec<Model>
}

impl ModelFactory {
    pub fn new(file_path: PathBuf) -> ModelFactory {
        ModelFactory {
            source_file_path: file_path,
            raw_model_data: RawModelData::default(),
            models: vec![]
        }
    }

    fn extract_next_model_from_stream(&mut self, model_name: String, lines_iter: &mut Lines) -> Option<String> {
        let mut model = Model::new(model_name);
        for l in lines_iter {
            let mut line_parts = l.split_whitespace();
            let key = match line_parts.next() {
                Some(k) => k,
                None => break
            };
            match key {
                KEY_VERTEX => {
                    let x: f32 = line_parts.next().unwrap().parse().unwrap();
                    let y: f32 = line_parts.next().unwrap().parse().unwrap();
                    let z: f32 = line_parts.next().unwrap().parse().unwrap();
                    self.raw_model_data.push_position([x, y, z]);
                },
                KEY_NORMAL => {
                    let x: f32 = line_parts.next().unwrap().parse().unwrap();
                    let y: f32 = line_parts.next().unwrap().parse().unwrap();
                    let z: f32 = line_parts.next().unwrap().parse().unwrap();
                    self.raw_model_data.push_normal([x, y, z]);
                },
                KEY_TEX_COORD => {
                    let s: f32 = line_parts.next().unwrap().parse().unwrap();
                    let t: f32 = line_parts.next().unwrap().parse().unwrap();
                    self.raw_model_data.push_tex_coord([s, t]);
                },
                KEY_FACE => {
                    let mut index_sets: Vec<IndexSet> = vec![];
                    while let Some(grouping) = line_parts.next() {
                        let first_slash = grouping.find('/').unwrap();
                        let second_slash = grouping.rfind('/').unwrap();
                        let position_index: u16 = grouping[0..first_slash].parse::<u16>().unwrap() - 1;
                        let tex_coord_index: u16 = grouping[(first_slash + 1)..second_slash].parse::<u16>().unwrap() - 1;
                        let normal_index: u16 = grouping[(second_slash + 1)..].parse::<u16>().unwrap() - 1;
                        index_sets.push(IndexSet { position_index, normal_index, tex_coord_index });
                    }

                    let start_index: u16 = {
                        let grouping = &index_sets[0];
                        let position = self.raw_model_data.get_raw_position(grouping.position_index).unwrap();
                        let normal = self.raw_model_data.get_raw_normal(grouping.normal_index).unwrap();
                        let tex_coord = self.raw_model_data.get_raw_tex_coord(grouping.tex_coord_index).unwrap();
                        let vertex = Vertex::from_components(&position, &normal, &tex_coord);
                        model.get_index(grouping.position_index as u64, grouping.normal_index as u64, grouping.tex_coord_index as u64, vertex)
                    };

                    let mut second_index: u16 = {
                        let grouping = &index_sets[1];
                        let position = self.raw_model_data.get_raw_position(grouping.position_index).unwrap();
                        let normal = self.raw_model_data.get_raw_normal(grouping.normal_index).unwrap();
                        let tex_coord = self.raw_model_data.get_raw_tex_coord(grouping.tex_coord_index).unwrap();
                        let vertex = Vertex::from_components(&position, &normal, &tex_coord);
                        model.get_index(grouping.position_index as u64, grouping.normal_index as u64, grouping.tex_coord_index as u64, vertex)
                    };

                    for grouping in index_sets.iter().take(index_sets.len()).skip(2) {
                        let position = self.raw_model_data.get_raw_position(grouping.position_index).unwrap();
                        let normal = self.raw_model_data.get_raw_normal(grouping.normal_index).unwrap();
                        let tex_coord = self.raw_model_data.get_raw_tex_coord(grouping.tex_coord_index).unwrap();
                        let vertex = Vertex::from_components(&position, &normal, &tex_coord);
                        let third_index = model.get_index(grouping.position_index as u64, grouping.normal_index as u64, grouping.tex_coord_index as u64, vertex);
                        model.add_face([start_index, second_index, third_index]);
                        second_index = third_index;
                    }
                },
                KEY_OBJECT => {
                    self.models.push(model);
                    let model_name = match line_parts.next() {
                        Some(name) => name,
                        None => panic!("No model name found!")
                    };
                    return Some(String::from(model_name));
                },
                _ => ()
            }
        }
        self.models.push(model);
        None
    }

    pub fn extract_all_models_from_file(&mut self) {
        let file_contents = fs::read_to_string(&self.source_file_path).unwrap();
        let mut lines_iter = file_contents.lines();
        while let Some(l) = lines_iter.next() {
            let line = l.trim();
            if line.is_empty() {
                continue;
            }
            let mut line_parts = line.split_whitespace();
            while let Some(part) = line_parts.next() {
                if part == KEY_OBJECT {
                    let mut model_name = match line_parts.next() {
                        Some(name) => String::from(name),
                        None => panic!("No model name found!")
                    };
                    loop {
                        model_name = match self.extract_next_model_from_stream(model_name, &mut lines_iter) {
                            Some(name) => name,
                            None => break
                        };
                    }
                    break;
                }
            }
        }
    }

    pub fn export_all_models(&self, dst_path: &Path) {
        println!("Files written:");
        for model in self.models.iter() {
            let mut output_file: PathBuf = dst_path.into();
            output_file.push(model.get_name());
            output_file.set_extension("mdl");
            let mut file = File::create(output_file).unwrap();
            let result = unsafe {
                model.write_data_to_file(&mut file)
            };
            match result {
                Ok(()) => println!(" {}.mdl", model.get_name()),
                _ => panic!("Error writing file: {}.mdl", model.get_name())
            }
        }
    }
}
