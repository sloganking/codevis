use code_visualizer::renderer;

fn main() {
    let paths = renderer::get_unicode_files_in_dir("./input/");

    // render files to image, and store in ./output.png
    renderer::render(&paths, 100, 16.0 / 9.0, true, true).save("./output.png").unwrap();
}
