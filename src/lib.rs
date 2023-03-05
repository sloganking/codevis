use anyhow::bail;
use prodash::Progress;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub mod render;
pub use render::function::render;

pub fn unicode_content(
    search_path: &Path,
    ignore_extensions: &[OsString],
    mut progress: impl Progress,
    should_interrupt: &AtomicBool,
) -> anyhow::Result<(Vec<(PathBuf, String)>, usize)> {
    let start = std::time::Instant::now();
    progress.init(None, Some(prodash::unit::label("files")));
    let mut content_progress = progress.add_child("content");
    content_progress.init(
        None,
        Some(prodash::unit::dynamic_and_mode(
            prodash::unit::Bytes,
            prodash::unit::display::Mode::with_throughput(),
        )),
    );

    let mut paths = Vec::new();
    let mut ignored = 0;
    for entry in ignore::Walk::new(search_path) {
        if should_interrupt.load(Ordering::Relaxed) {
            bail!("Cancelled by user")
        }
        progress.inc();
        let entry = entry?;
        let path = entry.path();
        if !ignore_extensions.is_empty()
            && path.extension().map_or(false, |ext| {
                ignore_extensions.iter().any(|extension| ext == extension)
            })
        {
            ignored += 1;
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(path) {
            content_progress.inc_by(content.len());
            paths.push((path.strip_prefix(search_path).unwrap().to_owned(), content));
        }
    }

    progress.show_throughput(start);
    content_progress.show_throughput(start);
    Ok((paths, ignored))
}
