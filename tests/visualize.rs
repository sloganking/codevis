use std::path::Path;
use std::sync::atomic::AtomicBool;

#[test]
fn renders_self() {
    let paths =
        code_visualizer::unicode_content(Path::new("./src/"), prodash::progress::Discard).unwrap();
    code_visualizer::render(
        &paths,
        100,
        false,
        1,
        16.0 / 9.0,
        true,
        prodash::progress::Discard,
        &AtomicBool::default(),
    )
    .unwrap();
    code_visualizer::render(
        &paths,
        100,
        true,
        2,
        16.0 / 9.0,
        true,
        prodash::progress::Discard,
        &AtomicBool::default(),
    )
    .unwrap();
}
