use std::path::Path;
use std::sync::atomic::AtomicBool;

#[test]
fn renders_self() {
    let (paths, ignored) = code_visualizer::unicode_content(
        Path::new("./src/"),
        &[],
        prodash::progress::Discard,
        &AtomicBool::default(),
    )
    .unwrap();
    assert_eq!(ignored, 0, "no ignore pattern configured");

    let theme = "Solarized (dark)";
    code_visualizer::render(
        &paths,
        100,
        false,
        1,
        16.0 / 9.0,
        true,
        theme,
        code_visualizer::FgColor::Style,
        code_visualizer::BgColor::Style,
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
        theme,
        code_visualizer::FgColor::StyleAsciiBrightness,
        code_visualizer::BgColor::HelixEditor,
        prodash::progress::Discard,
        &AtomicBool::default(),
    )
    .unwrap();
}
