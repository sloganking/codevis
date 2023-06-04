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
        message_buffer_capacity: if args.display_to_be_processed_file {
            200
        } else {
            20
        },
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
    let (mut dir_contents, mut ignored) = codevis::unicode_content(
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

    // filter extensions if there is a whitelist
    if !args.whitelist_extension.is_empty() {
        let mut whitelist_ignored: usize = 0;
        dir_contents.children_content.retain(|(path, _)| {
            path.extension().map_or(false, |ext| {
                if args.whitelist_extension.contains(&ext.to_owned()) {
                    true
                } else {
                    whitelist_ignored += 1;
                    false
                }
            })
        });
        ignored = whitelist_ignored;
    }

    dir_contents
        .children_content
        .sort_unstable_by(|(a, _), (b, _)| a.cmp(b));

    // log num ignored files
    if ignored != 0 {
        progress.add_child("input").info(format!(
            "Ignored {ignored} files that matched ignored extensions"
        ));
    }

    // determine themes to render files with
    let ts = ThemeSet::load_defaults();
    if args.all_themes {
        args.theme = ts.themes.keys().map(ToOwned::to_owned).collect();
    }

    let ss = SyntaxSet::load_defaults_newlines();
    for theme in &args.theme {
        let start = std::time::Instant::now();

        let img = codevis::render(
            &dir_contents,
            progress.add_child("render"),
            &should_interrupt,
            &ss,
            &ts,
            codevis::render::Options {
                column_width: args.column_width_pixels,
                line_height: args.line_height_pixels,
                readable: args.readable,
                show_filenames: args.show_filenames,
                target_aspect_ratio: args.aspect_width / args.aspect_height,
                threads: args.threads,
                highlight_truncated_lines: args.highlight_truncated_lines,
                force_full_columns: !args.dont_force_full_columns,
                plain: args.force_plain_syntax,
                display_to_be_processed_file: args.display_to_be_processed_file,
                theme,
                fg_color: if args.readable {
                    codevis::render::FgColor::Style
                } else {
                    args.fg_pixel_color
                },
                bg_color: args.bg_pixel_color,
                color_modulation: args.color_modulation,
                ignore_files_without_syntax: args.ignore_files_without_syntax,
                tab_spaces: args.tab_spaces,
                line_nums: args.line_nums,
            },
        )?;
        let img_path = if args.theme.len() == 1 {
            Cow::Borrowed(&args.output_path)
        } else {
            // mutate the output filename to include the theme in it.
            let mut extension = theme.replace(['(', ')'], "").replace(' ', "-");
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
