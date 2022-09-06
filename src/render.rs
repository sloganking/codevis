use image::Rgb;
use syntect::highlighting::Style;

/// Determine the foreground pixel color.
#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum FgColor {
    /// Use the style of the syntax to color the foreground pixel.
    Style,
    /// Encode the ascii value into the brightness of the style color
    StyleAsciiBrightness,
}

/// Determine the background pixel color.
#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum BgColor {
    /// Use the style of the syntax to color the background pixel.
    Style,
    /// The purple color of the Helix Editor.
    HelixEditor,
}

impl BgColor {
    pub fn to_rgb(&self, style: Style) -> Rgb<u8> {
        match self {
            BgColor::Style => Rgb([style.background.r, style.background.g, style.background.b]),
            BgColor::HelixEditor => Rgb([59, 34, 76]),
        }
    }
}

/// Configure how to render an image.
#[derive(Debug, Copy, Clone)]
pub struct Options<'a> {
    pub column_width: u32,
    pub line_height: u32,
    pub target_aspect_ratio: f64,

    pub fg_color: FgColor,
    pub bg_color: BgColor,
    pub theme: &'a str,

    pub force_full_columns: bool,
    pub ignore_files_without_syntax: bool,
}

pub(crate) mod function {
    use crate::render::{chunk, Options};
    use anyhow::{bail, Context};
    use image::{ImageBuffer, Pixel, Rgb};
    use memmap2::MmapMut;
    use prodash::Progress;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use syntect::highlighting::ThemeSet;
    use syntect::parsing::SyntaxSet;

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        content: &[(PathBuf, String)],
        mut progress: impl prodash::Progress,
        should_interrupt: &AtomicBool,
        Options {
            column_width,
            line_height,
            target_aspect_ratio,
            fg_color,
            bg_color,
            theme,
            force_full_columns,
            ignore_files_without_syntax,
        }: Options,
    ) -> anyhow::Result<ImageBuffer<Rgb<u8>, MmapMut>> {
        // unused for now
        // could be used to make a "rolling code" animation
        let start = std::time::Instant::now();

        let ss = SyntaxSet::load_defaults_newlines();

        //> read files (for /n counting)
        let (content, total_line_count, num_ignored) = {
            let mut out = Vec::with_capacity(content.len());
            let mut lines = 0;
            let mut num_ignored = 0;
            for (path, content) in content {
                let num_content_lines = content.lines().count();
                lines += num_content_lines;
                if ignore_files_without_syntax && ss.find_syntax_for_file(path)?.is_none() {
                    lines -= num_content_lines;
                    num_ignored += 1;
                } else {
                    out.push(((path, content), num_content_lines))
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

        let (mut img, lines_per_column, required_columns) = {
            //> determine image dimensions based on num of lines and constraints
            let mut lines_per_column = 1;
            let mut last_checked_aspect_ratio: f64 = f64::MAX;
            let mut last_column_line_limit = lines_per_column;
            let mut required_columns;
            let mut cur_aspect_ratio: f64 =
                column_width as f64 * total_line_count as f64 / (lines_per_column as f64 * 2.0);

            //<> determine maximum aspect ratios
            let tallest_aspect_ratio = column_width as f64 / total_line_count as f64 * 2.0;
            let widest_aspect_ratio = total_line_count as f64 * column_width as f64 / 2.0;
            //<

            if target_aspect_ratio <= tallest_aspect_ratio {
                //> use tallest possible aspect ratio
                lines_per_column = total_line_count;
                required_columns = 1;
                //<
            } else if target_aspect_ratio >= widest_aspect_ratio {
                //> use widest possible aspect ratio
                lines_per_column = 1;
                required_columns = total_line_count;
                //<
            } else {
                //> start at widest possible aspect ratio
                lines_per_column = 1;
                // required_columns = line_count;
                //<

                // de-widen aspect ratio until closest match is found
                while (last_checked_aspect_ratio - target_aspect_ratio).abs()
                    > (cur_aspect_ratio - target_aspect_ratio).abs()
                {
                    // remember current aspect ratio
                    last_checked_aspect_ratio = cur_aspect_ratio;

                    if force_full_columns {
                        last_column_line_limit = lines_per_column;

                        //> determine required number of columns
                        required_columns = total_line_count / lines_per_column;
                        if total_line_count % lines_per_column != 0 {
                            required_columns += 1;
                        }
                        //<

                        let last_required_columns = required_columns;

                        // find next full column aspect ratio
                        while required_columns == last_required_columns {
                            lines_per_column += 1;

                            //> determine required number of columns
                            required_columns = total_line_count / lines_per_column;
                            if total_line_count % lines_per_column != 0 {
                                required_columns += 1;
                            }
                            //<
                        }
                    } else {
                        //> generate new aspect ratio
                        lines_per_column += 1;

                        //> determine required number of columns
                        required_columns = total_line_count / lines_per_column;
                        if total_line_count % lines_per_column != 0 {
                            required_columns += 1;
                        }
                        //<
                        //<
                    }

                    cur_aspect_ratio = required_columns as f64 * column_width as f64
                        / (lines_per_column as f64 * line_height as f64);
                }

                //> re-determine best aspect ratio

                // (Should never not happen, but)
                // previous while loop would never have been entered if (column_line_limit == 1)
                // so (column_line_limit -= 1;) would be unnecessary
                if lines_per_column != 1 && !force_full_columns {
                    // revert to last aspect ratio
                    lines_per_column -= 1;
                } else if force_full_columns {
                    lines_per_column = last_column_line_limit;
                }

                //> determine required number of columns
                required_columns = total_line_count / lines_per_column;
                if total_line_count % lines_per_column != 0 {
                    required_columns += 1;
                }
            }

            let imgx: u32 = required_columns * column_width;
            let imgy: u32 = total_line_count.min(lines_per_column) * line_height;
            let channel_count = Rgb::<u8>::CHANNEL_COUNT;
            let num_pixels = imgx as usize * imgy as usize * channel_count as usize;
            progress.info(format!(
                "Image dimensions: {imgx} x {imgy} x {channel_count} ({} in virtual memory)",
                bytesize::ByteSize(num_pixels as u64)
            ));

            let img = ImageBuffer::<Rgb<u8>, _>::from_raw(
                imgx,
                imgy,
                memmap2::MmapMut::map_anon(num_pixels)?,
            )
            .expect("correct size computation above");

            progress.info(format!(
                "Aspect ratio is {} off from target",
                (last_checked_aspect_ratio - target_aspect_ratio).abs(),
            ));
            (img, lines_per_column, required_columns)
        };

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

        let ts = ThemeSet::load_defaults();
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
        let mut line_num: u32 = 0;
        let mut longest_line_chars = 0;
        let mut background = None;
        for ((path, content), num_content_lines) in content {
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

            let out = chunk::process(
                content,
                &mut img,
                |line| highlighter.highlight(line, &ss),
                chunk::Context {
                    column_width,
                    line_height,
                    total_line_count,
                    line_num,
                    lines_per_column,
                    fg_color,
                    bg_color,
                },
            );
            longest_line_chars = out.longest_line_in_chars.max(longest_line_chars);
            line_num += num_content_lines as u32;
            line_progress.inc_by(num_content_lines);
            background = out.background;
        }

        //> fill in any empty bottom right corner, with background color
        while line_num < lines_per_column * required_columns {
            let cur_y = (line_num % lines_per_column) * line_height;
            let cur_column_x_offset = (line_num / lines_per_column) * column_width;
            let background = background.unwrap_or(Rgb([0, 0, 0]));

            for cur_line_x in 0..column_width {
                for y_pos in cur_y..cur_y + line_height {
                    img.put_pixel(cur_column_x_offset + cur_line_x, y_pos, background);
                }
            }
            line_num += 1;
        }

        progress.show_throughput(start);
        line_progress.show_throughput(start);
        progress.info(format!(
            "Longest encountered line in chars: {longest_line_chars}"
        ));
        if num_ignored != 0 {
            progress.info(format!("Ignored {num_ignored} files due to missing syntax",))
        }

        Ok(img)
    }
}

#[allow(dead_code)]
mod chunk {
    use crate::render::{BgColor, FgColor};
    use bstr::ByteSlice;
    use image::{ImageBuffer, Rgb};
    use std::ops::{Deref, DerefMut};
    use syntect::highlighting::Style;

    /// The result of processing a chunk.
    pub struct Outcome {
        /// The longest line we encountered in unicode codepoints.
        pub longest_line_in_chars: usize,
        /// The last used background color
        pub background: Option<Rgb<u8>>,
    }

    pub struct Context {
        pub column_width: u32,
        pub line_height: u32,
        pub total_line_count: u32,
        pub line_num: u32,
        pub lines_per_column: u32,

        pub fg_color: FgColor,
        pub bg_color: BgColor,
    }

    pub fn process<C>(
        content: &str,
        img: &mut ImageBuffer<Rgb<u8>, C>,
        mut highlight: impl FnMut(&str) -> Vec<(Style, &str)>,
        Context {
            column_width,
            line_height,
            total_line_count,
            mut line_num,
            lines_per_column,
            fg_color,
            bg_color,
        }: Context,
    ) -> Outcome
    where
        C: Deref<Target = [u8]>,
        C: DerefMut,
    {
        let mut longest_line_in_chars = 0;
        let mut background = None;
        for line in content.as_bytes().lines_with_terminator() {
            let line = line.to_str().expect("UTF-8 was source");
            longest_line_in_chars = longest_line_in_chars.max(line.chars().count());

            let actual_line = line_num % total_line_count;
            let cur_y = (actual_line % lines_per_column) * line_height;
            let cur_column_x_offset = (actual_line / lines_per_column) * column_width;

            let regions = highlight(line);
            let background = background.get_or_insert_with(|| bg_color.to_rgb(regions[0].0));
            let mut cur_line_x = 0;

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
                        FgColor::Style => {
                            Rgb([style.foreground.r, style.foreground.g, style.foreground.b])
                        }
                        FgColor::StyleAsciiBrightness => {
                            let fg_byte = (chr as usize) & 0xff;
                            let boost = 2.4;
                            Rgb([
                                (((fg_byte * style.foreground.r as usize) as f32 / u16::MAX as f32)
                                    * boost
                                    * 256.0) as u8,
                                (((fg_byte * style.foreground.g as usize) as f32 / u16::MAX as f32)
                                    * boost
                                    * 256.0) as u8,
                                (((fg_byte * style.foreground.b as usize) as f32 / u16::MAX as f32)
                                    * boost
                                    * 256.0) as u8,
                            ])
                        }
                    };

                    if chr == ' ' || chr == '\n' || chr == '\r' {
                        for y_pos in cur_y..cur_y + line_height {
                            img.put_pixel(cur_column_x_offset + cur_line_x, y_pos, *background);
                        }

                        cur_line_x += 1;
                    } else if chr == '\t' {
                        let tab_spaces = 4;
                        let spaces_to_add = tab_spaces - (cur_line_x % tab_spaces);

                        for _ in 0..spaces_to_add {
                            if cur_line_x >= column_width {
                                break;
                            }

                            for y_pos in cur_y..cur_y + line_height {
                                img.put_pixel(cur_column_x_offset + cur_line_x, y_pos, *background);
                            }

                            cur_line_x += 1;
                        }
                    } else {
                        for y_pos in cur_y..cur_y + line_height {
                            img.put_pixel(cur_column_x_offset + cur_line_x, y_pos, char_color);
                        }

                        cur_line_x += 1;
                    }
                }
            }

            while cur_line_x < column_width {
                for y_pos in cur_y..cur_y + line_height {
                    img.put_pixel(cur_column_x_offset + cur_line_x, y_pos, *background);
                }

                cur_line_x += 1;
            }

            line_num += 1;
        }

        Outcome {
            longest_line_in_chars,
            background,
        }
    }
}
