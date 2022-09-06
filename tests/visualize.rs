use bstr::ByteSlice;
use code_visualizer::render;
use std::path::Path;
use std::sync::atomic::AtomicBool;

#[test]
fn various_renders() {
    let (paths, ignored) = code_visualizer::unicode_content(
        Path::new("./src/"),
        &[],
        prodash::progress::Discard,
        &AtomicBool::default(),
    )
    .unwrap();
    assert_eq!(ignored, 0, "no ignore pattern configured");

    let theme = "Solarized (dark)";
    let mut opts = render::Options {
        column_width: 100,
        line_height: 1,
        target_aspect_ratio: 0.0,
        fg_color: code_visualizer::render::FgColor::Style,
        bg_color: code_visualizer::render::BgColor::Style,
        threads: 1,
        theme,
        force_full_columns: false,
        ignore_files_without_syntax: true,
    };
    code_visualizer::render(
        paths.clone(),
        prodash::progress::Discard,
        &AtomicBool::default(),
        opts,
    )
    .unwrap();

    opts.force_full_columns = true;
    opts.ignore_files_without_syntax = false;
    opts.line_height = 2;
    opts.fg_color = code_visualizer::render::FgColor::StyleAsciiBrightness;
    opts.bg_color = code_visualizer::render::BgColor::HelixEditor;
    opts.threads = 3;
    opts.target_aspect_ratio = 16.0 / 9.0;

    code_visualizer::render(
        paths.clone(),
        prodash::progress::Discard,
        &AtomicBool::default(),
        opts,
    )
    .unwrap();

    opts.line_height = 2;
    code_visualizer::render(
        paths,
        prodash::progress::Discard,
        &AtomicBool::default(),
        opts,
    )
    .unwrap();
}

#[test]
#[ignore]
fn multi_threading_produces_same_result_as_single_threaded_mode() {
    let (paths, ignored) = code_visualizer::unicode_content(
        Path::new("./src/"),
        &[],
        prodash::progress::Discard,
        &AtomicBool::default(),
    )
    .unwrap();
    assert_eq!(ignored, 0, "no ignore pattern configured");

    let theme = "Solarized (dark)";
    let mut opts = render::Options {
        column_width: 100,
        line_height: 1,
        target_aspect_ratio: 0.0,
        fg_color: code_visualizer::render::FgColor::Style,
        bg_color: code_visualizer::render::BgColor::Style,
        threads: 1,
        theme,
        force_full_columns: false,
        ignore_files_without_syntax: true,
    };
    let expected = code_visualizer::render(
        paths.clone(),
        prodash::progress::Discard,
        &AtomicBool::default(),
        opts,
    )
    .unwrap();

    opts.threads = 2;
    let actual = code_visualizer::render(
        paths.clone(),
        prodash::progress::Discard,
        &AtomicBool::default(),
        opts,
    )
    .unwrap();
    assert!(
        actual.as_bytes() == expected.as_bytes(),
        "multi-threaded version should be pixel-perfect"
    );
}
