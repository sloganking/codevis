use std::sync::atomic::AtomicBool;

#[test]
fn renders_self() {
    let paths = code_visualizer::renderer::get_unicode_files_in_dir("./src/").unwrap();
    code_visualizer::renderer::render(
        &paths,
        100,
        16.0 / 9.0,
        true,
        prodash::progress::Discard,
        &AtomicBool::default(),
    )
    .unwrap();
}
