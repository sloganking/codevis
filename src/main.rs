use code_visualizer::renderer;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

fn main() {
    let paths = renderer::get_unicode_files_in_dir("./input/");
    let should_interrupt = Arc::new(AtomicBool::new(false));
    let _ = signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&should_interrupt));

    {
        let progress: Arc<prodash::Tree> = prodash::TreeOptions {
            message_buffer_capacity: 200,
            ..Default::default()
        }
        .into();

        let render = prodash::render::line(
            std::io::stderr(),
            std::sync::Arc::downgrade(&progress),
            prodash::render::line::Options {
                frames_per_second: 24.0,
                initial_delay: None,
                timestamp: true,
                throughput: true,
                hide_cursor: true,
                ..prodash::render::line::Options::default()
            }
            .auto_configure(prodash::render::line::StreamKind::Stderr),
        );

        // render files to image, and store in ./output.png
        renderer::render(
            &paths,
            100,
            16.0 / 9.0,
            true,
            progress.add_child("render"),
            &should_interrupt,
        )
        .expect("No UTF-8 compatible files found.")
        .save("./output.png")
        .expect("Failed to save PNG file");
        render
    }
    .wait()
}
