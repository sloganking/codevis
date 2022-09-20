use anyhow::Context;
use image::{ImageBuffer, Rgb};
use memmap2::MmapMut;
use std::borrow::Cow;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

mod options;

fn main() -> anyhow::Result<()> {
    let mut args: options::Args = clap::Parser::parse();

    let should_interrupt = Arc::new(AtomicBool::new(false));
    let _ = signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&should_interrupt));

    let progress: Arc<prodash::Tree> = prodash::TreeOptions {
        message_buffer_capacity: args.display_to_be_processed_file.then(|| 200).unwrap_or(20),
        ..Default::default()
    }
    .into();

    let render_progress = prodash::render::line(
        std::io::stderr(),
        Arc::downgrade(&progress),
        prodash::render::line::Options {
            frames_per_second: 24.0,
            initial_delay: None,
            timestamp: false,
            throughput: true,
            hide_cursor: true,
            level_filter: Some(0..=2),
            ..prodash::render::line::Options::default()
        }
        .auto_configure(prodash::render::line::StreamKind::Stderr),
    );

    // determine files to render
    let (paths, ignored) = codevis::unicode_content(
        &args.input_dir,
        &args.ignore_extension,
        progress.add_child("search unicode files"),
        &should_interrupt,
    )
    .with_context(|| {
        format!(
            "Failed to find input files in {:?} directory",
            args.input_dir
        )
    })?;
    if ignored != 0 {
        progress.add_child("input").info(format!(
            "Ignored {ignored} files that matched ignored extensions"
        ));
    }

    // determine themes to render files with
    let ts = ThemeSet::load_defaults();
    if args.all_themes {
        assert!(
            args.theme.is_empty(),
            "BUG: CLI shouldn't allow to pass custom themes when --all-themes is set"
        );
        args.theme = ts.themes.keys().map(ToOwned::to_owned).collect();
    }
    if args.theme.is_empty() {
        args.theme.push("Solarized (dark)".into());
    }

    let ss = SyntaxSet::load_defaults_newlines();
    for theme in &args.theme {
        let start = std::time::Instant::now();

        let img = codevis::render(
            &paths,
            progress.add_child("render"),
            &should_interrupt,
            &ss,
            &ts,
            codevis::render::Options {
                column_width: args.column_width_pixels,
                line_height: args.line_height_pixels,
                target_aspect_ratio: args.aspect_width / args.aspect_height,
                threads: args.threads,
                highlight_truncated_lines: args.highlight_truncated_lines,
                force_full_columns: !args.dont_force_full_columns,
                plain: args.force_plain_syntax,
                display_to_be_processed_file: args.display_to_be_processed_file,
                theme,
                fg_color: args.fg_pixel_color,
                bg_color: args.bg_pixel_color,
                color_modulation: args.color_modulation,
                ignore_files_without_syntax: args.ignore_files_without_syntax,
            },
        )?;
        let img_path = if args.theme.len() == 1 {
            Cow::Borrowed(&args.output_path)
        } else {
            // mutate the output filename to include the theme in it.
            let mut extension = theme.replace('(', "").replace(')', "").replace(' ', "-");
            extension.push('.');
            extension.push_str(
                args.output_path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .context("Output paths needs an extension")?,
            );
            let theme_specific_path = args.output_path.with_extension(extension);
            Cow::Owned(theme_specific_path)
        };
        sage_image(
            img,
            img_path.as_ref(),
            progress.add_child(format!(
                "saving {}",
                img_path
                    .as_ref()
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("")
            )),
        )?;

        if args.open {
            progress
                .add_child("opening")
                .info(img_path.display().to_string());
            open::that(img_path.as_ref())?;
        }
        progress.add_child("operation").done(format!(
            "done in {:.02}s",
            std::time::Instant::now()
                .checked_duration_since(start)
                .unwrap_or_default()
                .as_secs_f32()
        ));
    }

    render_progress.shutdown_and_wait();
    Ok(())
}

fn sage_image(
    img: ImageBuffer<Rgb<u8>, MmapMut>,
    img_path: &Path,
    mut progress: impl prodash::Progress,
) -> anyhow::Result<()> {
    let start = std::time::Instant::now();
    progress.init(
        Some(img.width() as usize * img.height() as usize * 3),
        Some(prodash::unit::dynamic_and_mode(
            prodash::unit::Bytes,
            prodash::unit::display::Mode::with_throughput(),
        )),
    );

    // There is no image format that can reasonably stream arbitrary image formats, so writing
    // isn't interactive.
    // I think the goal would be to write a TGA file (it can handle huge files in theory while being uncompressed)
    // and write directly into a memory map on disk, or any other format that can.
    // In the mean time, PNG files work as well even though some apps are buggy with these image resolutions.
    img.save(img_path)?;
    let bytes = img_path
        .metadata()
        .map_or(0, |md| md.len() as prodash::progress::Step);
    progress.inc_by(bytes);
    progress.show_throughput(start);
    Ok(())
}
