use anyhow::bail;
use prodash::Progress;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

// The number of lines used for displaying filenames at
// the top of files.
const FILENAME_LINE_COUNT: u32 = 1;
pub mod render;
pub use render::function::render;

pub struct DirContents {
    pub parent_dir: PathBuf,
    pub children_content: Vec<(PathBuf, String)>,
}

pub fn unicode_content(
    search_path: &Path,
    ignore_extensions: &[OsString],
    mut progress: impl Progress,
    should_interrupt: &AtomicBool,
) -> anyhow::Result<(DirContents, usize)> {
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
            paths.push((path.to_owned(), content));
        }
    }

    progress.show_throughput(start);
    content_progress.show_throughput(start);
    Ok((
        DirContents {
            parent_dir: search_path.to_path_buf(),
            children_content: paths,
        },
        ignored,
    ))
}
