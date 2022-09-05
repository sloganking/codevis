use anyhow::Context;
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
    let res = code_visualizer::render(
        &paths,
        args.column_width_pixels,
        args.ignore_files_without_syntax,
        args.line_height_pixels,
        args.aspect_width / args.aspect_height,
        args.force_full_columns,
        &args.theme,
        args.fg_pixel_color,
        args.bg_pixel_color,
        progress.add_child("render"),
        &should_interrupt,
    );
    render_progress.shutdown_and_wait();
    res?.save(&args.output_path)?;

    if args.open {
        open::that(&args.output_path)?;
    }

    Ok(())
}
