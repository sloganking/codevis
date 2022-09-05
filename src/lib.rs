use anyhow::{bail, Context};
use bstr::ByteSlice;
use image::{ImageBuffer, Rgb, RgbImage};
use prodash::Progress;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;

#[allow(clippy::too_many_arguments)]
pub fn render(
    content: &[(PathBuf, String)],
    column_width: u32,
    ignore_files_without_syntax: bool,
    line_height: u32,
    target_aspect_ratio: f64,
    force_full_columns: bool,
    theme: &str,
    mut progress: impl prodash::Progress,
    should_interrupt: &AtomicBool,
) -> anyhow::Result<ImageBuffer<Rgb<u8>, Vec<u8>>> {
    // unused for now
    // could be used to make a "rolling code" animation
    let line_offset = 0;
    let start = std::time::Instant::now();

    let ss = SyntaxSet::load_defaults_newlines();

    //> read files (for /n counting)
    let (total_line_count, ignored) = {
        let mut lines = 0;
        let mut ignored = BTreeSet::default();
        for (path, content) in content {
            let content_lines = content.lines().count();
            lines += content_lines;
            if ignore_files_without_syntax && ss.find_syntax_for_file(path)?.is_none() {
                lines -= content_lines;
                ignored.insert(path);
            }
        }
        (lines as u32, ignored)
    };

    if total_line_count == 0 {
        bail!(
            "Did not find a single line to render in {} files",
            content.len()
        );
    }

    //> determine image dimensions based on num of lines and constraints
    let mut last_checked_aspect_ratio: f64 = f64::MAX;
    let mut column_line_limit = 1;
    let mut last_column_line_limit = column_line_limit;
    let mut required_columns;
    let mut cur_aspect_ratio: f64 =
        column_width as f64 * total_line_count as f64 / (column_line_limit as f64 * 2.0);

    //<> determine maximum aspect ratios
    let tallest_aspect_ratio = column_width as f64 / total_line_count as f64 * 2.0;
    let widest_aspect_ratio = total_line_count as f64 * column_width as f64 / 2.0;
    //<

    if target_aspect_ratio <= tallest_aspect_ratio {
        //> use tallest possible aspect ratio
        column_line_limit = total_line_count;
        required_columns = 1;
        //<
    } else if target_aspect_ratio >= widest_aspect_ratio {
        //> use widest possible aspect ratio
        column_line_limit = 1;
        required_columns = total_line_count;
        //<
    } else {
        //> start at widest possible aspect ratio
        column_line_limit = 1;
        // required_columns = line_count;
        //<

        // de-widen aspect ratio until closest match is found
        while (last_checked_aspect_ratio - target_aspect_ratio).abs()
            > (cur_aspect_ratio - target_aspect_ratio).abs()
        {
            // remember current aspect ratio
            last_checked_aspect_ratio = cur_aspect_ratio;

            if force_full_columns {
                last_column_line_limit = column_line_limit;

                //> determine required number of columns
                required_columns = total_line_count / column_line_limit;
                if total_line_count % column_line_limit != 0 {
                    required_columns += 1;
                }
                //<

                let last_required_columns = required_columns;

                // find next full column aspect ratio
                while required_columns == last_required_columns {
                    column_line_limit += 1;

                    //> determine required number of columns
                    required_columns = total_line_count / column_line_limit;
                    if total_line_count % column_line_limit != 0 {
                        required_columns += 1;
                    }
                    //<
                }
            } else {
                //> generate new aspect ratio
                column_line_limit += 1;

                //> determine required number of columns
                required_columns = total_line_count / column_line_limit;
                if total_line_count % column_line_limit != 0 {
                    required_columns += 1;
                }
                //<
                //<
            }

            cur_aspect_ratio = required_columns as f64 * column_width as f64
                / (column_line_limit as f64 * line_height as f64);
        }

        //> re-determine best aspect ratio

        // (Should never not happen, but)
        // previous while loop would never have been entered if (column_line_limit == 1)
        // so (column_line_limit -= 1;) would be unnecessary
        if column_line_limit != 1 && !force_full_columns {
            // revert to last aspect ratio
            column_line_limit -= 1;
        } else if force_full_columns {
            column_line_limit = last_column_line_limit;
        }

        //> determine required number of columns
        required_columns = total_line_count / column_line_limit;
        if total_line_count % column_line_limit != 0 {
            required_columns += 1;
        }
    }

    //> remake immutable
    let required_columns = required_columns;
    let column_line_limit = column_line_limit;
    //<

    //<> initialize image
    //> determine x
    let imgx: u32 = required_columns * column_width;

    //<> determine y
    let imgy: u32 = if total_line_count < column_line_limit {
        total_line_count * line_height
    } else {
        column_line_limit * line_height
    };
    //<
    progress.info(format!(
        "Image dimensions: {imgx} x {imgy} ({} in memory)",
        bytesize::ByteSize(imgx as u64 * imgy as u64)
    ));

    // Create a new ImgBuf with width: imgx and height: imgy
    let mut imgbuf = RgbImage::new(imgx, imgy);

    // render all lines onto image
    progress.set_name("overall");
    progress.init(
        Some(content.len() - ignored.len()),
        prodash::unit::label_and_mode("files", prodash::unit::display::Mode::with_percentage())
            .into(),
    );
    let mut line_progress = progress.add_child("render");
    line_progress.init(
        Some(total_line_count as usize),
        prodash::unit::label_and_mode("lines", prodash::unit::display::Mode::with_throughput())
            .into(),
    );

    //> initialize highlighting themes
    let ts = ThemeSet::load_defaults();

    //<> initialize rendering vars
    let mut cur_line_x = 0;
    let mut line_num: u32 = 0;
    let mut background = Rgb([0, 0, 0]);
    let mut longest_line_chars = 0;

    for (path, content) in content {
        if ignored.contains(path) {
            continue;
        }
        progress.inc();
        if should_interrupt.load(Ordering::Relaxed) {
            bail!("Cancelled by user")
        }

        let mut highlighter = syntect::easy::HighlightLines::new(
            ss.find_syntax_for_file(path)?
                .unwrap_or_else(|| ss.find_syntax_plain_text()),
            ts.themes.get(theme).with_context(|| {
                format!(
                    "Could not find theme {theme:?}, must be one of {}",
                    ts.themes
                        .keys()
                        .map(|s| format!("{s:?}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?,
        );

        for line in content.as_bytes().lines_with_terminator() {
            let line = line.to_str().expect("UTF-8 was source");
            longest_line_chars = longest_line_chars.max(line.chars().count());

            line_progress.inc();
            {
                //> get position of current line
                let actual_line = (line_num + line_offset) % total_line_count;
                let cur_y = (actual_line % column_line_limit) * line_height;
                let cur_column_x_offset = (actual_line / column_line_limit) * column_width;

                let regions: Vec<(Style, &str)> = highlighter.highlight(line, &ss);

                background = Rgb([
                    regions[0].0.background.r,
                    regions[0].0.background.g,
                    regions[0].0.background.b,
                ]);

                for region in regions {
                    if cur_line_x >= column_width {
                        break;
                    }

                    let char_color: Rgb<u8> = Rgb([
                        region.0.foreground.r,
                        region.0.foreground.g,
                        region.0.foreground.b,
                    ]);

                    for chr in region.1.chars() {
                        if cur_line_x >= column_width || region.1.chars().count() == 0 {
                            break;
                        }

                        //> place pixel for character
                        if chr == ' ' || chr == '\n' || chr == '\r' {
                            for y_pos in cur_y..cur_y + line_height {
                                imgbuf.put_pixel(
                                    cur_column_x_offset + cur_line_x,
                                    y_pos,
                                    background,
                                );
                            }

                            cur_line_x += 1;
                        } else if chr == '\t' {
                            // specifies how many spaces a tab should be rendered as
                            let tab_spaces = 4;

                            let spaces_to_add = tab_spaces - (cur_line_x % tab_spaces);

                            for _ in 0..spaces_to_add {
                                if cur_line_x >= column_width {
                                    break;
                                }

                                for y_pos in cur_y..cur_y + line_height {
                                    imgbuf.put_pixel(
                                        cur_column_x_offset + cur_line_x,
                                        y_pos,
                                        background,
                                    );
                                }

                                cur_line_x += 1;
                            }
                        } else {
                            for y_pos in cur_y..cur_y + line_height {
                                imgbuf.put_pixel(
                                    cur_column_x_offset + cur_line_x,
                                    y_pos,
                                    char_color,
                                );
                            }

                            cur_line_x += 1;
                        }
                        //<
                    }
                }

                while cur_line_x < column_width {
                    for y_pos in cur_y..cur_y + line_height {
                        imgbuf.put_pixel(cur_column_x_offset + cur_line_x, y_pos, background);
                    }

                    cur_line_x += 1;
                }

                cur_line_x = 0;
                line_num += 1;
            } // until NLL this scope is needed so we can clear the buffer after
        }
    }

    //> fill in any empty bottom right corner, with background color
    while line_num < column_line_limit * required_columns {
        let cur_y = (line_num % column_line_limit) * line_height;
        let cur_column_x_offset = (line_num / column_line_limit) * column_width;

        //<> fill line with background color
        for cur_line_x in 0..column_width {
            for y_pos in cur_y..cur_y + line_height {
                imgbuf.put_pixel(cur_column_x_offset + cur_line_x, y_pos, background);
            }
        }
        line_num += 1;
    }

    progress.show_throughput(start);
    line_progress.show_throughput(start);
    progress.info(format!(
        "Longest encountered line in chars: {longest_line_chars}"
    ));
    progress.info(format!(
        "Aspect ratio is {} off from target",
        (last_checked_aspect_ratio - target_aspect_ratio).abs(),
    ));
    if !ignored.is_empty() {
        progress.info(format!(
            "Ignored {} files due to missing syntax",
            ignored.len()
        ))
    }

    Ok(imgbuf)
}

pub fn unicode_content(
    path: &Path,
    ignore_extensions: &[OsString],
    mut progress: impl Progress,
) -> anyhow::Result<(Vec<(PathBuf, String)>, usize)> {
    progress.init(None, Some(prodash::unit::label("files")));

    let mut paths = Vec::new();
    let mut ignored = 0;
    for entry in ignore::Walk::new(path) {
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
