use anyhow::bail;
use image::{ImageBuffer, Pixel, Rgb, RgbImage};
use memmap2::MmapMut;
use prodash::Progress;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub mod render;
pub use render::function::render;

// pub mod tilecache;
pub use render::tilecache::TileCache;

pub enum RenderType {
    TileCache(TileCache),
    MmapImage(ImageBuffer<Rgb<u8>, MmapMut>),
    Image(ImageBuffer<Rgb<u8>, Vec<u8>>),
}

impl RenderType {
    pub fn new_tilecache(path: PathBuf, img_width: usize) -> Self {
        Self::TileCache(TileCache::new(path, (img_width / 256) + 2))
    }

    pub fn new_mmap_image(width: u32, height: u32) -> Self {
        let channel_count = Rgb::<u8>::CHANNEL_COUNT;
        let num_pixels = width as usize * height as usize * channel_count as usize;

        let mut mmap_img = ImageBuffer::<Rgb<u8>, _>::from_raw(
            width,
            height,
            MmapMut::map_anon(num_pixels).expect("Failed to allocate memmap"),
        )
        .expect("correct size computation above");

        Self::MmapImage(mmap_img)
    }

    pub fn new_image(width: u32, height: u32) -> Self {
        Self::Image(RgbImage::new(width, height))
    }

    fn put_pixel(&mut self, x: i32, y: i32, pixel: Rgb<u8>) {
        match self {
            RenderType::TileCache(cache) => {
                // rgb to rgba
                let pixel = image::Rgba([pixel[0], pixel[1], pixel[2], 255]);
                cache.put_pixel(x, y, pixel);
            }
            RenderType::MmapImage(img) => img.put_pixel(x as u32, y as u32, pixel),
            RenderType::Image(img) => img.put_pixel(x as u32, y as u32, pixel),
        }
    }

    fn get_pixel(&mut self, x: i32, y: i32) -> Rgb<u8> {
        match self {
            RenderType::TileCache(cache) => {
                let rgba_pixel = cache.get_pixel(x, y);
                // rgba to rgb
                Rgb([rgba_pixel[0], rgba_pixel[1], rgba_pixel[2]])
            }
            RenderType::MmapImage(img) => *img.get_pixel(x as u32, y as u32),
            RenderType::Image(img) => *img.get_pixel(x as u32, y as u32),
        }
    }
}

// The number of lines used for displaying filenames at
// the top of files.
const FILENAME_LINE_COUNT: u32 = 1;

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
