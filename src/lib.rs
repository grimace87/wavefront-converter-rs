pub mod collisiondata;
pub mod model;
pub mod modelfactory;

use std::fs;
use std::path::PathBuf;
use modelfactory::ModelFactory;

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::process_directory;
    use crate::model::{Model, Vertex};
    use crate::collisiondata::CollisionData;
    use std::fs::File;
    use std::io::Read;

    fn expected_vertex_data() -> Vec<Vertex> {
        vec![
            Vertex::from_components(&[1.0, 2.0, -1.0], &[0.0, 1.0, 0.0], &[0.625, 0.5]),
            Vertex::from_components(&[-1.0, 2.0, -1.0], &[0.0, 1.0, 0.0], &[0.875, 0.5]),
            Vertex::from_components(&[-1.0, 2.0, 1.0], &[0.0, 1.0, 0.0], &[0.875, 0.75]),
            Vertex::from_components(&[1.0, 2.0, 1.0], &[0.0, 1.0, 0.0], &[0.625, 0.75]),
            Vertex::from_components(&[1.0, 0.0, 1.0], &[0.0, 0.0, 1.0], &[0.375, 0.75]),
            Vertex::from_components(&[1.0, 2.0, 1.0], &[0.0, 0.0, 1.0], &[0.625, 0.75]),
            Vertex::from_components(&[-1.0, 2.0, 1.0], &[0.0, 0.0, 1.0], &[0.625, 1.0]),
            Vertex::from_components(&[-1.0, 0.0, 1.0], &[0.0, 0.0, 1.0], &[0.375, 1.0]),
            Vertex::from_components(&[-1.0, 0.0, 1.0], &[-1.0, 0.0, 0.0], &[0.375, 0.0]),
            Vertex::from_components(&[-1.0, 2.0, 1.0], &[-1.0, 0.0, 0.0], &[0.625, 0.0]),
            Vertex::from_components(&[-1.0, 2.0, -1.0], &[-1.0, 0.0, 0.0], &[0.625, 0.25]),
            Vertex::from_components(&[-1.0, 0.0, -1.0], &[-1.0, 0.0, 0.0], &[0.375, 0.25]),
            Vertex::from_components(&[-1.0, 0.0, -1.0], &[0.0, -1.0, 0.0], &[0.125, 0.5]),
            Vertex::from_components(&[1.0, 0.0, -1.0], &[0.0, -1.0, 0.0], &[0.375, 0.5]),
            Vertex::from_components(&[1.0, 0.0, 1.0], &[0.0, -1.0, 0.0], &[0.375, 0.75]),
            Vertex::from_components(&[-1.0, 0.0, 1.0], &[0.0, -1.0, 0.0], &[0.125, 0.75]),
            Vertex::from_components(&[1.0, 0.0, -1.0], &[1.0, 0.0, 0.0], &[0.375, 0.5]),
            Vertex::from_components(&[1.0, 2.0, -1.0], &[1.0, 0.0, 0.0], &[0.625, 0.5]),
            Vertex::from_components(&[1.0, 2.0, 1.0], &[1.0, 0.0, 0.0], &[0.625, 0.75]),
            Vertex::from_components(&[1.0, 0.0, 1.0], &[1.0, 0.0, 0.0], &[0.375, 0.75]),
            Vertex::from_components(&[-1.0, 0.0, -1.0], &[0.0, 0.0, -1.0], &[0.375, 0.25]),
            Vertex::from_components(&[-1.0, 2.0, -1.0], &[0.0, 0.0, -1.0], &[0.625, 0.25]),
            Vertex::from_components(&[1.0, 2.0, -1.0], &[0.0, 0.0, -1.0], &[0.625, 0.5]),
            Vertex::from_components(&[1.0, 0.0, -1.0], &[0.0, 0.0, -1.0], &[0.375, 0.5]),
        ]
    }

    fn expected_index_data() -> Vec<u16> {
        vec![
            0, 1, 2, 0, 2, 3,
            4, 5, 6, 4, 6, 7,
            8, 9, 10, 8, 10, 11,
            12, 13, 14, 12, 14, 15,
            16, 17, 18, 16, 18, 19,
            20, 21, 22, 20, 22, 23
        ]
    }

    #[test]
    fn transcode_and_read_back_various_models() {
        // Parses models in resources/tests/variation, including a single-model file and a file
        // with two models in it

        let mut model_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        model_directory.push("resources");
        model_directory.push("tests");
        model_directory.push("variation");
        let mut output_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        output_directory.push("resources");
        output_directory.push("models");
        if !output_directory.is_dir() {
            std::fs::create_dir(&output_directory).unwrap();
        }
        process_directory(&model_directory, &output_directory, None);

        for entry in std::fs::read_dir(output_directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if let Some(ext) = path.extension() {
                if ext.eq("mdl") {
                    println!("Processing file: {}", path.as_os_str().to_str().unwrap());

                    let mut file = File::open(&path).unwrap();

                    let metadata = std::fs::metadata(&path).unwrap();
                    let size_bytes = metadata.len() as usize;

                    let mut bytes = vec![0u8; size_bytes];
                    file.read_exact(bytes.as_mut_slice()).unwrap();
                    let model = unsafe { Model::from_bytes(bytes.as_slice()) };
                    println!("Read back model: {:?}", model);
                }
            }
        }
    }

    #[test]
    fn scrutinise_cube_model() {
        // Transcodes the Cube model, parses the output, and scrutinises the resulting Model struct

        let mut model_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        model_directory.push("resources");
        model_directory.push("tests");
        model_directory.push("scrutiny");
        let mut output_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        output_directory.push("resources");
        output_directory.push("models");
        if !output_directory.is_dir() {
            std::fs::create_dir(&output_directory).unwrap();
        }
        process_directory(&model_directory, &output_directory, None);

        let mut model_file_path = output_directory;
        model_file_path.push("Cube.mdl");
        assert!(model_file_path.is_file());
        let mut file = File::open(&model_file_path).unwrap();
        let metadata = std::fs::metadata(&model_file_path).unwrap();
        let size_bytes = metadata.len() as usize;
        let mut bytes = vec![0u8; size_bytes];
        file.read_exact(bytes.as_mut_slice()).unwrap();
        let model = unsafe { Model::from_bytes(bytes.as_slice()) };

        assert_eq!(model.interleaved_vertices.len(), 24); // 3 unique vertices per corner (3 possible normals)
        assert_eq!(model.face_indices.len(), 36);
        assert_eq!(model.interleaved_vertices, expected_vertex_data());
        assert_eq!(model.face_indices, expected_index_data());
    }

    #[test]
    fn scrutinise_enclosure_collisions() {
        // Transcodes the Enclosure model, including generating collision data, then parses the
        // generated collisions file and checks for the expected data

        let mut model_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        model_directory.push("resources");
        model_directory.push("tests");
        model_directory.push("closed");
        let mut model_output_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        model_output_directory.push("resources");
        model_output_directory.push("models");
        if !model_output_directory.is_dir() {
            std::fs::create_dir(&model_output_directory).unwrap();
        }
        let mut collision_output_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        collision_output_directory.push("resources");
        collision_output_directory.push("models");
        if !collision_output_directory.is_dir() {
            std::fs::create_dir(&collision_output_directory).unwrap();
        }
        process_directory(&model_directory, &model_output_directory, Some(&collision_output_directory));

        let mut collision_file_path = collision_output_directory;
        collision_file_path.push("Enclosure.csn");
        assert!(collision_file_path.is_file());
        let mut file = File::open(&collision_file_path).unwrap();
        let metadata = std::fs::metadata(&collision_file_path).unwrap();
        let size_bytes = metadata.len() as usize;
        let mut bytes = vec![0u8; size_bytes];
        file.read_exact(bytes.as_mut_slice()).unwrap();
        let collision_data = unsafe { CollisionData::from_bytes(bytes.as_slice()) };

        assert_eq!(collision_data.extent_x, [-3.0, 5.25]);
        assert_eq!(collision_data.extent_y, [0.0, 4.0]);
        assert_eq!(collision_data.extent_z, [-5.0, 3.0]);
        assert_eq!(collision_data.traction_surfaces.len(), 18);
        assert_eq!(collision_data.sliding_surfaces.len(), 2);
        assert_eq!(collision_data.walls.len(), 18);
    }
}

pub fn process_directory(src_path: &PathBuf, dst_path: &PathBuf, collisions_dst_path: Option<&PathBuf>) {
    println!("Processing models in directory {:?}: ", src_path);
    for entry in fs::read_dir(src_path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let extension = match path.extension() {
            Some(e) => e,
            None => continue
        };
        match extension.to_str() {
            Some("obj") => process_file(path, dst_path, collisions_dst_path),
            _ => continue
        };
    }
    println!("Models successfully processed");
}

fn process_file(src_file_path: PathBuf, dst_path: &PathBuf, collisions_dst_path: Option<&PathBuf>) {
    let mut factory = ModelFactory::new(src_file_path);
    let include_collisions = collisions_dst_path.is_some();
    factory.extract_all_models_from_file(include_collisions);
    factory.export_all(dst_path, collisions_dst_path);
}
