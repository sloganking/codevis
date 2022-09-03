use std::{env, process};
use code_visualizer::renderer;

fn main() {
    let mut code_dir = env::args()
        .nth(1)
        .unwrap_or_else(|| {
            println!("USAGE: code-visualizer <directory>");
            process::exit(1);
        });
    if !code_dir.ends_with('/') {
        code_dir.push('/');
    }
    let paths = renderer::get_unicode_files_in_dir(&code_dir);

    // render files to image, and store in ./output.png
    renderer::render(&paths, 100, 16.0 / 9.0, true, true)
        .expect("No UTF-8 compatible files found.")
        .save("./output.png")
        .expect("Failed to save PNG file");
}
