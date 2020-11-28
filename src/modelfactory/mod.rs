use std::fs;
use std::path::PathBuf;
use std::str::Lines;

use crate::model::{RawModelData, Model, Vertex};
use std::fs::File;
use crate::collisiondata::{CollisionData, Surface, Vec3, WALL_NORMAL_ELEVATION_MIN, WALL_NORMAL_ELEVATION_MAX, SLIDE_NORMAL_ELEVATION_MIN, SLIDE_NORMAL_ELEVATION_MAX, Wall};

pub const FILE_VERSION_NUMBER: u32 = 1;

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
    models: Vec<Model>,
    collision_data: Vec<CollisionData>
}

impl ModelFactory {
    pub fn new(file_path: PathBuf) -> ModelFactory {
        ModelFactory {
            source_file_path: file_path,
            raw_model_data: RawModelData::default(),
            models: vec![],
            collision_data: vec![]
        }
    }

    /// Find the Vertex data for an index set (panics if the vertex data isn't found)
    fn vertex_from_indices(&self, indices: &IndexSet) -> Vertex {
        let position = self.raw_model_data.get_raw_position(indices.position_index).unwrap();
        let normal = self.raw_model_data.get_raw_normal(indices.normal_index).unwrap();
        let tex_coord = self.raw_model_data.get_raw_tex_coord(indices.tex_coord_index).unwrap();
        Vertex::from_components(&position, &normal, &tex_coord)
    }

    /// Given n index sets, generate n-2 faces (triangles)
    fn add_faces_for_index_sets(&self, index_sets: &Vec<IndexSet>, model: &mut Model) {
        let start_index: u16 = {
            let grouping = &index_sets[0];
            let vertex = self.vertex_from_indices(grouping);
            model.get_index(grouping.position_index as u64, grouping.normal_index as u64, grouping.tex_coord_index as u64, vertex)
        };

        let mut second_index: u16 = {
            let grouping = &index_sets[1];
            let vertex = self.vertex_from_indices(grouping);
            model.get_index(grouping.position_index as u64, grouping.normal_index as u64, grouping.tex_coord_index as u64, vertex)
        };

        for grouping in index_sets.iter().take(index_sets.len()).skip(2) {
            let vertex = self.vertex_from_indices(grouping);
            let third_index = model.get_index(grouping.position_index as u64, grouping.normal_index as u64, grouping.tex_coord_index as u64, vertex);
            model.add_face([start_index, second_index, third_index]);
            second_index = third_index;
        }
    }

    /// If there are 3 or 4 index sets, generate collision data
    /// Angle of the normal determines whether to form sliding or traction surfaces (one per triangle)
    /// or walls (one per quad if possible, else one per triangle). Since triangles may form quads
    /// without being stored in the source data as quads, the final wall data should be passed over
    /// to merge walls that look to be duplicates of each other.
    fn add_collisions_for_index_sets(&self, index_sets: &Vec<IndexSet>, collision_data: &mut CollisionData) {

        let vertices: Vec<Vertex> = index_sets.iter()
            .map(|set| self.vertex_from_indices(set))
            .collect();
        let polygon_count = vertices.len() as isize - 2;
        if polygon_count < 1 {
            return;
        }
        let polygon_count = polygon_count as usize;

        let mut all_surfaces: Vec<(Surface, f32)> = vec![];
        for i in 0..polygon_count {
            let vertex_0: &Vertex = &vertices[0];
            let vertex_1: &Vertex = &vertices[i + 1];
            let vertex_2: &Vertex = &vertices[i + 2];
            let average_normal = {
                let x = (vertex_0.normal[0] + vertex_1.normal[0] + vertex_2.normal[0]) / 3.0;
                let y = (vertex_0.normal[1] + vertex_1.normal[1] + vertex_2.normal[1]) / 3.0;
                let z = (vertex_0.normal[2] + vertex_1.normal[2] + vertex_2.normal[2]) / 3.0;
                Vec3 { x, y, z }
            };
            let surface = Surface {
                point_0: Vec3 { x: vertex_0.position[0], y: vertex_0.position[1], z: vertex_0.position[2] },
                point_1: Vec3 { x: vertex_1.position[0], y: vertex_1.position[1], z: vertex_1.position[2] },
                point_2: Vec3 { x: vertex_2.position[0], y: vertex_2.position[1], z: vertex_2.position[2] },
                normal: average_normal
            };
            let normal_elevation = {
                let normal_length = (average_normal.x * average_normal.x + average_normal.y * average_normal.y + average_normal.z * average_normal.z).sqrt();
                (average_normal.y / normal_length).asin()
            };
            all_surfaces.push((surface, normal_elevation));
        }

        // Check for special case where there are 2 polygons and they both have a wall-oriented normal
        let make_wall_from_quad = if all_surfaces.len() == 2 {
            let angle_1 = all_surfaces[0].1;
            let angle_2 = all_surfaces[1].1;
            angle_1 > WALL_NORMAL_ELEVATION_MIN && angle_1 < WALL_NORMAL_ELEVATION_MAX && angle_2 > WALL_NORMAL_ELEVATION_MIN && angle_2 < WALL_NORMAL_ELEVATION_MAX
        } else {
            false
        };

        // Make the wall for the above case if needed
        if make_wall_from_quad {
            let points: [&Vec3; 4] = [
                &all_surfaces[0].0.point_0,
                &all_surfaces[0].0.point_1,
                &all_surfaces[1].0.point_1,
                &all_surfaces[1].0.point_2
            ];
            let mut approx_wall_normal = (all_surfaces[0].0.normal + all_surfaces[1].0.normal) * 0.5;
            approx_wall_normal.y = 0.0;
            let left_direction = Vec3 { x: -approx_wall_normal.z, y: 0.0, z: approx_wall_normal.x };
            let right_direction = Vec3 { x: approx_wall_normal.z, y: 0.0, z: -approx_wall_normal.x };
            let mut left_extreme_point = *points[Self::max_point_of_4_in_direction(&left_direction, &points)];
            left_extreme_point.y = Self::min_of_4(points[0].y, points[1].y, points[2].y, points[3].y);
            let mut right_extreme_point = *points[Self::max_point_of_4_in_direction(&right_direction, &points)];
            right_extreme_point.y = Self::max_of_4(points[0].y, points[1].y, points[2].y, points[3].y);
            let wall = Wall::from_bottom_left_to_top_right(left_extreme_point, right_extreme_point);
            collision_data.walls.push(wall);
            return
        }

        // For each polygon, add to collision data whatever kind of wall or surface it is
        for surface in all_surfaces.iter() {
            let angle = surface.1;
            if angle > WALL_NORMAL_ELEVATION_MIN && angle < WALL_NORMAL_ELEVATION_MAX {
                let points: [&Vec3; 3] = [
                    &surface.0.point_0,
                    &surface.0.point_1,
                    &surface.0.point_2
                ];
                let mut approx_wall_normal = surface.0.normal;
                approx_wall_normal.y = 0.0;
                let left_direction = Vec3 { x: -approx_wall_normal.z, y: 0.0, z: approx_wall_normal.x };
                let right_direction = Vec3 { x: approx_wall_normal.z, y: 0.0, z: -approx_wall_normal.x };
                let mut left_extreme_point = *points[Self::max_point_of_3_in_direction(&left_direction, &points)];
                left_extreme_point.y = Self::min_of_3(points[0].y, points[1].y, points[2].y);
                let mut right_extreme_point = *points[Self::max_point_of_3_in_direction(&right_direction, &points)];
                right_extreme_point.y = Self::max_of_3(points[0].y, points[1].y, points[2].y);
                let wall = Wall::from_bottom_left_to_top_right(left_extreme_point, right_extreme_point);
                collision_data.walls.push(wall);
            } else if angle < SLIDE_NORMAL_ELEVATION_MAX && angle > SLIDE_NORMAL_ELEVATION_MIN {
                collision_data.sliding_surfaces.push(surface.0);
            } else {
                collision_data.traction_surfaces.push(surface.0);
            }
        }
    }

    fn min_of_3(val0: f32, val1: f32, val2: f32) -> f32 {
        let mut min_value = val0;
        if val1 < min_value {
            min_value = val1;
        }
        if val2 < min_value {
            min_value = val2;
        }
        min_value
    }

    fn max_of_3(val0: f32, val1: f32, val2: f32) -> f32 {
        let mut min_value = val0;
        if val1 > min_value {
            min_value = val1;
        }
        if val2 > min_value {
            min_value = val2;
        }
        min_value
    }

    fn min_of_4(val0: f32, val1: f32, val2: f32, val3: f32) -> f32 {
        let mut min_value = val0;
        if val1 < min_value {
            min_value = val1;
        }
        if val2 < min_value {
            min_value = val2;
        }
        if val3 < min_value {
            min_value = val3;
        }
        min_value
    }

    fn max_of_4(val0: f32, val1: f32, val2: f32, val3: f32) -> f32 {
        let mut min_value = val0;
        if val1 > min_value {
            min_value = val1;
        }
        if val2 > min_value {
            min_value = val2;
        }
        if val3 > min_value {
            min_value = val3;
        }
        min_value
    }

    fn max_point_of_3_in_direction(direction: &Vec3, points: &[&Vec3; 3]) -> usize {
        let dots: [f32; 3] = [
            points[0].dot(direction),
            points[1].dot(direction),
            points[2].dot(direction)
        ];
        let mut max_index = 0;
        let mut max_value = dots[0];
        if dots[1] > max_value {
            max_index = 1;
            max_value = dots[1];
        }
        if dots[2] > max_value {
            max_index = 2;
        }
        max_index
    }

    fn max_point_of_4_in_direction(direction: &Vec3, points: &[&Vec3; 4]) -> usize {
        let dots: [f32; 4] = [
            points[0].dot(direction),
            points[1].dot(direction),
            points[2].dot(direction),
            points[3].dot(direction)
        ];
        let mut max_index = 0;
        let mut max_value = dots[0];
        if dots[1] > max_value {
            max_index = 1;
            max_value = dots[1];
        }
        if dots[2] > max_value {
            max_index = 2;
            max_value = dots[2];
        }
        if dots[3] > max_value {
            max_index = 3;
        }
        max_index
    }

    fn extract_next_model_from_stream(&mut self, model_name: String, lines_iter: &mut Lines, include_collisions: bool) -> Option<String> {
        let mut model = Model::new(model_name.clone());
        let mut collision_data = CollisionData::new(model_name);
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

                    self.add_faces_for_index_sets(&index_sets, &mut model);
                    if include_collisions {
                        self.add_collisions_for_index_sets(&index_sets, &mut collision_data);
                    }
                },
                KEY_OBJECT => {
                    self.models.push(model);
                    self.collision_data.push(collision_data);
                    let model_name = match line_parts.next() {
                        Some(name) => name,
                        None => panic!("No model name found!")
                    };
                    return Some(String::from(model_name));
                },
                _ => ()
            }
        }
        collision_data.remove_wall_duplicates();
        collision_data.find_extents();
        self.models.push(model);
        self.collision_data.push(collision_data);
        None
    }

    pub fn extract_all_models_from_file(&mut self, include_collisions: bool) {
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
                        model_name = match self.extract_next_model_from_stream(model_name, &mut lines_iter, include_collisions) {
                            Some(name) => name,
                            None => break
                        };
                    }
                    break;
                }
            }
        }
    }

    pub fn export_all(&self, dst_path: &PathBuf, collision_maps_path: Option<&PathBuf>) {
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

        if collision_maps_path.is_none() {
            return
        }
        let collision_dir = collision_maps_path.unwrap();
        for collisions in self.collision_data.iter() {
            let mut output_file: PathBuf = collision_dir.into();
            output_file.push(collisions.get_model_name());
            output_file.set_extension("csn");
            let mut file = File::create(output_file).unwrap();

            let result = unsafe {
                collisions.write_data_to_file(&mut file)
            };
            match result {
                Ok(()) => println!(" {}.csn", collisions.get_model_name()),
                _ => panic!("Error writing file: {}.csn", collisions.get_model_name())
            }
        }
    }
}
