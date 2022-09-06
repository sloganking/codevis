/// Determine the foreground pixel color.
#[derive(clap::ValueEnum, Clone, Debug)]
pub enum FgColor {
    /// Use the style of the syntax to color the foreground pixel.
    Style,
    /// Encode the ascii value into the brightness of the style color
    StyleAsciiBrightness,
}

/// Determine the background pixel color.
#[derive(clap::ValueEnum, Clone, Debug)]
pub enum BgColor {
    /// Use the style of the syntax to color the background pixel.
    Style,
    /// The purple color of the Helix Editor.
    HelixEditor,
}

pub(crate) mod function {
    use crate::render::{BgColor, FgColor};
    use anyhow::{bail, Context};
    use bstr::ByteSlice;
    use image::{ImageBuffer, Pixel, Rgb};
    use memmap2::MmapMut;
    use prodash::Progress;
    use std::path::PathBuf;
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
        fg_color: FgColor,
        bg_color: BgColor,
        mut progress: impl prodash::Progress,
        should_interrupt: &AtomicBool,
    ) -> anyhow::Result<ImageBuffer<Rgb<u8>, MmapMut>> {
        // unused for now
        // could be used to make a "rolling code" animation
        let line_offset = 0;
        let start = std::time::Instant::now();

        let ss = SyntaxSet::load_defaults_newlines();

        //> read files (for /n counting)
        let (content, total_line_count, num_ignored) = {
            let mut out = Vec::with_capacity(content.len());
            let mut lines = 0;
            let mut num_ignored = 0;
            for (path, content) in content {
                let content_lines = content.lines().count();
                lines += content_lines;
                if ignore_files_without_syntax && ss.find_syntax_for_file(path)?.is_none() {
                    lines -= content_lines;
                    num_ignored += 1;
                } else {
                    out.push(((path, content), content_lines))
                }
            }
            (out, lines as u32, num_ignored)
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
            "Image dimensions: {imgx} x {imgy} x 3 ({} in virtual memory)",
            bytesize::ByteSize(imgx as u64 * imgy as u64 * 3)
        ));

        // Create a new ImgBuf with width: imgx and height: imgy
        let mut imgbuf = ImageBuffer::<Rgb<u8>, _>::from_raw(
            imgx,
            imgy,
            memmap2::MmapMut::map_anon(
                imgx as usize * imgy as usize * Rgb::<u8>::CHANNEL_COUNT as usize,
            )?,
        )
        .expect("correct size computation above");

        // render all lines onto image
        progress.set_name("overall");
        progress.init(
            Some(content.len()),
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
        let mut prev_syntax = ss.find_syntax_plain_text() as *const _;
        let theme = ts.themes.get(theme).with_context(|| {
            format!(
                "Could not find theme {theme:?}, must be one of {}",
                ts.themes
                    .keys()
                    .map(|s| format!("{s:?}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
        let mut highlighter =
            syntect::easy::HighlightLines::new(ss.find_syntax_plain_text(), theme);

        for ((path, content), _content_lines) in content {
            progress.inc();
            if should_interrupt.load(Ordering::Relaxed) {
                bail!("Cancelled by user")
            }

            let syntax = ss
                .find_syntax_for_file(path)?
                .unwrap_or_else(|| ss.find_syntax_plain_text());
            if syntax as *const _ != prev_syntax {
                highlighter = syntect::easy::HighlightLines::new(syntax, theme);
                prev_syntax = syntax as *const _;
            }

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

                    background = match bg_color {
                        BgColor::Style => Rgb([
                            regions[0].0.background.r,
                            regions[0].0.background.g,
                            regions[0].0.background.b,
                        ]),
                        BgColor::HelixEditor => Rgb([59, 34, 76]),
                    };

                    for (style, region) in regions {
                        if cur_line_x >= column_width {
                            break;
                        }
                        if region.is_empty() {
                            continue;
                        }

                        for chr in region.chars() {
                            if cur_line_x >= column_width {
                                break;
                            }

                            let char_color: Rgb<u8> = match fg_color {
                                FgColor::Style => Rgb([
                                    style.foreground.r,
                                    style.foreground.g,
                                    style.foreground.b,
                                ]),
                                FgColor::StyleAsciiBrightness => {
                                    let fg_byte = (chr as usize) & 0xff;
                                    let boost = 2.4;
                                    Rgb([
                                        (((fg_byte * style.foreground.r as usize) as f32
                                            / u16::MAX as f32)
                                            * boost
                                            * 256.0) as u8,
                                        (((fg_byte * style.foreground.g as usize) as f32
                                            / u16::MAX as f32)
                                            * boost
                                            * 256.0) as u8,
                                        (((fg_byte * style.foreground.b as usize) as f32
                                            / u16::MAX as f32)
                                            * boost
                                            * 256.0) as u8,
                                    ])
                                }
                            };

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
        if num_ignored != 0 {
            progress.info(format!("Ignored {num_ignored} files due to missing syntax",))
        }

        Ok(imgbuf)
    }
}

#[allow(dead_code)]
mod chunk {
    use image::{ImageBuffer, Rgb};
    use std::path::Path;

    /// Essentially a rectangle of pixels in memory, with an embedded offset to change the starting position
    /// into the bigger picture.
    pub struct Frame<C> {
        /// The underlying buffer
        buf: ImageBuffer<Rgb<u8>, C>,

        /// The amount of pixels per line in horizontal direction.
        column_width: u32,
        /// The amount of pixels per line
        line_height: u32,
        /// The amount of lines.
        lines: u32,

        /// Offset in pixels along with width, always a multiple of column-width.
        x_ofs: u32,
        /// The starting line of the column
        y_ofs: u32,
    }

    /// The result of processing a chunk.
    pub struct Outcome<C> {
        pub frame: Frame<C>,
        /// The longest line we encountered in unicode codepoints.
        pub longest_line_in_chars: u32,
    }

    /// A piece of work to process
    pub struct Work<'a> {
        /// The path from which `content` was read
        pub path: &'a Path,
        /// The UTF-8 content at `path`
        pub content: &'a str,
        /// The starting position in x (in pixels) of the parent image
        pub start_x_pos: u32,
        /// The starting position in y (in lines) of the parent image, with line-height taken into account
        pub start_y_pos: u32,
    }

    pub fn process<C>(
        _work: Work,
        _highlighter: &mut syntect::easy::HighlightLines<'_>,
    ) -> Outcome<C> {
        todo!()
    }
}
