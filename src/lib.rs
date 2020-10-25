pub mod model;
pub mod modelfactory;

use std::fs;
use std::path::PathBuf;
use modelfactory::ModelFactory;

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::process_directory;
    use crate::model::Model;
    use std::fs::File;
    use std::io::Read;

    #[test]
    fn process_and_read_back_models() {
        let mut model_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        model_directory.push("resources");
        model_directory.push("rawmodels");
        let mut output_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        output_directory.push("resources");
        output_directory.push("models");
        if !output_directory.is_dir() {
            std::fs::create_dir(&output_directory).unwrap();
        }
        process_directory(&model_directory, &output_directory);

        for entry in std::fs::read_dir(output_directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                println!("Processing file: {}", path.as_os_str().to_str().unwrap());

                let mut file = File::open(&path).unwrap();

                let metadata = std::fs::metadata(&path).unwrap();
                let size_bytes = metadata.len() as usize;

                let mut bytes = vec![0u8; size_bytes];
                file.read_exact(bytes.as_mut_slice()).unwrap();
                let model = unsafe { Model::from_bytes(&bytes) };
                println!("Read back model: {:?}", model);
            }
        }
    }
}

pub fn process_directory(src_path: &PathBuf, dst_path: &PathBuf) {
    println!("Processing models in directory {:?}: ", src_path);
    for entry in fs::read_dir(src_path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let extension = match path.extension() {
            Some(e) => e,
            None => continue
        };
        match extension.to_str() {
            Some("obj") => process_file(path, dst_path),
            _ => continue
        };
    }
    println!("Models successfully processed");
}

fn process_file(src_file_path: PathBuf, dst_path: &PathBuf) {
    let mut factory = ModelFactory::new(src_file_path, true, true);
    factory.print_status_message();
    factory.extract_all_models_from_file();
    factory.export_all_models(dst_path);
}
