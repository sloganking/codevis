use anyhow::bail;
use prodash::Progress;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

mod render;
pub use render::{render, BgColor, FgColor};

pub fn unicode_content(
    path: &Path,
    ignore_extensions: &[OsString],
    mut progress: impl Progress,
    should_interrupt: &AtomicBool,
) -> anyhow::Result<(Vec<(PathBuf, String)>, usize)> {
    progress.init(None, Some(prodash::unit::label("files")));

    let mut paths = Vec::new();
    let mut ignored = 0;
    for entry in ignore::Walk::new(path) {
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
            paths.push((path.to_owned(), content));
        }
    }

    Ok((paths, ignored))
}
