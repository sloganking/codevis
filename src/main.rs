use std::path::PathBuf;
use std::{fs};
use glob::glob;
use code_visualizer::renderer;

fn main() {

    // get list of valid files
        // get list of files in ./input/ using glob
            let mut paths = Vec::new();

            let file_delimiter = "";
            let search_params = String::from("./input/**/*") + file_delimiter;

            for entry in glob(&search_params).expect("Failed to read glob pattern") {
                match entry {
                    Ok(path) => {
                        paths.push(path);
                    },
                    Err(e) => println!("{:?}", e),
                }
            }

        // filter out directories
            let paths = paths.into_iter().filter(|e| e.is_file());

        // filter out non unicode files
            let paths: Vec<PathBuf> = paths.into_iter().filter(|e| {
                fs::read_to_string(e).is_ok()
            }).collect();

    renderer::render(&paths, 100, 16.0 / 9.0, true).save("./output.png").unwrap();
    
}
