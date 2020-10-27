use std::env;

extern crate wavefront_converter_rs;
use wavefront_converter_rs::process_directory;

fn main() {

    // First arg is executable path, verify second is present
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Relative file name must be provided as the first argument");
        return
    }

    let file_name = &args[1];
    let mut input_path = env::current_dir().unwrap();
    for segment in file_name.split("/") {
        if segment == "." {
            input_path.pop();
        } else {
            input_path.push(segment);
        }
    }

    let output_path = env::current_dir().unwrap();
    process_directory(&input_path, &output_path);
}
