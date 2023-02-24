use bstr::ByteSlice;
use codevis::render;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

#[test]
fn various_renders() {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let (paths, ignored) = codevis::unicode_content(
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
        plain: false,
        highlight_truncated_lines: true,
        display_to_be_processed_file: false,
        fg_color: codevis::render::FgColor::Style,
        bg_color: codevis::render::BgColor::Style,
        color_modulation: 0.2,
        threads: 1,
        theme,
        force_full_columns: false,
        ignore_files_without_syntax: true,
        tab_spaces: 4,
        readable: false,
    };
    codevis::render(
        &paths,
        prodash::progress::Discard,
        &AtomicBool::default(),
        &ss,
        &ts,
        opts,
    )
    .unwrap();

    opts.force_full_columns = true;
    opts.ignore_files_without_syntax = false;
    opts.line_height = 2;
    opts.highlight_truncated_lines = false;
    opts.fg_color = codevis::render::FgColor::StyleAsciiBrightness;
    opts.bg_color = codevis::render::BgColor::HelixEditor;
    opts.plain = true;
    opts.target_aspect_ratio = 16.0 / 9.0;

    codevis::render(
        &paths,
        prodash::progress::Discard,
        &AtomicBool::default(),
        &ss,
        &ts,
        opts,
    )
    .unwrap();

    opts.line_height = 2;
    codevis::render(
        &paths,
        prodash::progress::Discard,
        &AtomicBool::default(),
        &ss,
        &ts,
        opts,
    )
    .unwrap();
}

#[test]
fn multi_threading_produces_same_result_as_single_threaded_mode() {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let (paths, ignored) = codevis::unicode_content(
        Path::new("./src/"),
        &[],
        prodash::progress::Discard,
        &AtomicBool::default(),
    )
    .unwrap();
    assert_eq!(ignored, 0, "no ignore pattern configured");

    let theme = "Solarized (light)";
    let mut opts = render::Options {
        column_width: 100,
        line_height: 1,
        target_aspect_ratio: 0.0,
        highlight_truncated_lines: false,
        display_to_be_processed_file: true,
        plain: true,
        fg_color: codevis::render::FgColor::Style,
        bg_color: codevis::render::BgColor::Style,
        threads: 1,
        theme,
        color_modulation: 0.2,
        force_full_columns: false,
        ignore_files_without_syntax: true,
        tab_spaces: 4,
        readable: false,
    };
    let expected = codevis::render(
        &paths,
        prodash::progress::Discard,
        &AtomicBool::default(),
        &ss,
        &ts,
        opts,
    )
    .unwrap();

    opts.threads = 2;
    let actual = codevis::render(
        &paths,
        prodash::progress::Discard,
        &AtomicBool::default(),
        &ss,
        &ts,
        opts,
    )
    .unwrap();
    assert!(
        actual.as_bytes() == expected.as_bytes(),
        "multi-threaded version should be pixel-perfect"
    );
}
