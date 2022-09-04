use std::sync::atomic::AtomicBool;
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    let should_interrupt = Arc::new(AtomicBool::new(false));
    let _ = signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&should_interrupt));

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
            throughput: false,
            hide_cursor: true,
            ..prodash::render::line::Options::default()
        }
        .auto_configure(prodash::render::line::StreamKind::Stderr),
    );

    let paths =
        code_visualizer::unicode_content("./input/", progress.add_child("search unicode files"))?;
    let res = code_visualizer::render(
        &paths,
        100,
        16.0 / 9.0,
        true,
        progress.add_child("render"),
        &should_interrupt,
    );
    render.shutdown_and_wait();

    res?.save("./output.png")?;
    Ok(())
}
