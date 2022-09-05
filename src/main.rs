use anyhow::Context;
use bstr::ByteSlice;
use image::ImageEncoder;
use prodash::Progress;
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
    let img = code_visualizer::render(
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
    )?;

    let start = std::time::Instant::now();
    let mut progress = progress.add_child("saving");
    progress.init(
        Some(img.width() as usize * img.height() as usize),
        Some(prodash::unit::dynamic_and_mode(
            prodash::unit::Bytes,
            prodash::unit::display::Mode::with_throughput(),
        )),
    );

    if args.output_path.extension() == Some(std::ffi::OsStr::new("png")) {
        let mut out = util::WriteProgress {
            inner: std::io::BufWriter::new(std::fs::File::create(&args.output_path)?),
            progress,
        };
        image::codecs::png::PngEncoder::new(&mut out).write_image(
            img.as_bytes(),
            img.width(),
            img.height(),
            image::ColorType::Rgb8,
        )?;
        progress = out.progress;
    } else {
        img.save(&args.output_path)?;
        let bytes = args
            .output_path
            .metadata()
            .map_or(0, |md| md.len() as prodash::progress::Step);
        progress.inc_by(bytes);
    }
    progress.show_throughput(start);
    render_progress.shutdown_and_wait();

    if args.open {
        open::that(&args.output_path)?;
    }

    Ok(())
}

mod util {
    pub struct WriteProgress<W, P> {
        pub inner: W,
        pub progress: P,
    }

    impl<W, P> std::io::Write for WriteProgress<W, P>
    where
        W: std::io::Write,
        P: prodash::Progress,
    {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let bytes = self.inner.write(buf)?;
            self.progress.inc_by(bytes);
            Ok(bytes)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.inner.flush()
        }
    }
}
