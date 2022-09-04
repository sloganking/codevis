use std::sync::atomic::AtomicBool;

#[test]
fn renders_self() {
    let paths = code_visualizer::unicode_content("./src/", prodash::progress::Discard).unwrap();
    code_visualizer::render(
        &paths,
        100,
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
        2,
        16.0 / 9.0,
        true,
        prodash::progress::Discard,
        &AtomicBool::default(),
    )
    .unwrap();
}
