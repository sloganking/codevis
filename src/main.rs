use code_visualizer::renderer;

fn main() {

    // get list of valid files in ./input/
    let paths = renderer::get_unicode_files_in_dir("./input/");

    // render files to image, and store in ./output.png
    renderer::render(&paths, 100, 16.0 / 9.0, true).save("./output.png").unwrap();
}
