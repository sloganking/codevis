use anyhow::Context;
use image::{ImageBuffer, Rgb};
use memmap2::MmapMut;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

mod options;

fn main() -> anyhow::Result<()> {
    let args: options::Args = clap::Parser::parse();

    let should_interrupt = Arc::new(AtomicBool::new(false));
    let _ = signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&should_interrupt));

    let progress: Arc<prodash::Tree> = prodash::TreeOptions {
        message_buffer_capacity: 20,
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
            ..prodash::render::line::Options::default()
        }
        .auto_configure(prodash::render::line::StreamKind::Stderr),
    );

    let (paths, ignored) = code_visualizer::unicode_content(
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

    let start = std::time::Instant::now();
    let img = code_visualizer::render(
        paths,
        progress.add_child("render"),
        &should_interrupt,
        code_visualizer::render::Options {
            column_width: args.column_width_pixels,
            line_height: args.line_height_pixels,
            target_aspect_ratio: args.aspect_width / args.aspect_height,
            threads: args.threads,
            highlight_truncated_lines: args.highlight_truncated_lines,
            force_full_columns: args.force_full_columns,
            plain: args.plain,
            theme: &args.theme,
            fg_color: args.fg_pixel_color,
            bg_color: args.bg_pixel_color,
            ignore_files_without_syntax: args.ignore_files_without_syntax,
        },
    )?;

    let img_path = &args.output_path;
    sage_image(img, img_path, progress.add_child("saving"))?;

    if args.open {
        progress
            .add_child("opening")
            .info(img_path.display().to_string());
        open::that(img_path)?;
    }
    progress.add_child("operation").done(format!(
        "done in {:.02}s",
        std::time::Instant::now()
            .checked_duration_since(start)
            .unwrap_or_default()
            .as_secs_f32()
    ));

    render_progress.shutdown_and_wait();
    Ok(())
}

fn sage_image(
    img: ImageBuffer<Rgb<u8>, MmapMut>,
    img_path: &PathBuf,
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
