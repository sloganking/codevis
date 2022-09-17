use crate::render::chunk::calc_offsets;
use crate::render::Dimension;
use crate::render::{chunk, Options};
use anyhow::{bail, Context};
use image::{ImageBuffer, Pixel, Rgb, RgbImage};
use memmap2::MmapMut;
use prodash::Progress;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

pub fn render(
    content: Vec<(PathBuf, String)>,
    mut progress: impl prodash::Progress,
    should_interrupt: &AtomicBool,
    Options {
        column_width,
        line_height,
        target_aspect_ratio,
        threads,
        fg_color,
        bg_color,
        highlight_truncated_lines,
        display_to_be_processed_file,
        themes,
        force_full_columns,
        plain,
        ignore_files_without_syntax,
        color_modulation,
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
            if ignore_files_without_syntax && ss.find_syntax_for_file(&path)?.is_none() {
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

    // determine number and height of columns closest to desired aspect ratio
    let Dimension {
        imgx,
        imgy,
        lines_per_column,
        required_columns,
    } = crate::render::dimension::compute(
        target_aspect_ratio,
        column_width,
        total_line_count,
        line_height,
        force_full_columns,
        progress.add_child("determine dimensions"),
    )?;

    let num_pixels = {
        let channel_count = Rgb::<u8>::CHANNEL_COUNT;
        let num_pixels = imgx as usize * imgy as usize * channel_count as usize;
        let theme_count = themes.len().max(1);
        progress.info(format!(
            "Image dimensions: {imgx} x {imgy} x {channel_count} x {theme_count} [x * y * channels * themes] ({} in virtual memory)",
            bytesize::ByteSize(num_pixels as u64 * theme_count as u64),
        ));
        num_pixels
    };

    let mut img =
        ImageBuffer::<Rgb<u8>, _>::from_raw(imgx, imgy, memmap2::MmapMut::map_anon(num_pixels)?)
            .expect("correct size computation above");

    progress.set_name("process");
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
    let themes: Vec<_> = if themes.is_empty() {
        vec![ts.themes.get("Solarized (dark)").expect("built-in")]
    } else {
        themes
            .iter()
            .map(|theme| {
                ts.themes.get(theme).with_context(|| {
                    format!(
                        "Could not find theme {theme:?}, must be one of {}",
                        ts.themes
                            .keys()
                            .map(|s| format!("{s:?}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })
            })
            .collect::<Result<_, _>>()?
    };
    let theme = &themes[0]; // TODO: figure out what state is per theme actually.

    let threads = (threads == 0)
        .then(num_cpus::get)
        .unwrap_or(threads)
        .clamp(1, num_cpus::get());
    let (mut line_num, longest_line_chars, background) = if threads < 2 {
        let mut line_num: u32 = 0;
        let mut longest_line_chars = 0;
        let mut background = None;
        let mut highlighter =
            syntect::easy::HighlightLines::new(ss.find_syntax_plain_text(), theme);
        for (file_index, ((path, content), num_content_lines)) in content.into_iter().enumerate() {
            progress.inc();
            if should_interrupt.load(Ordering::Relaxed) {
                bail!("Cancelled by user")
            }

            if !plain {
                let syntax = ss
                    .find_syntax_for_file(&path)?
                    .unwrap_or_else(|| ss.find_syntax_plain_text());
                if syntax as *const _ != prev_syntax {
                    highlighter = syntect::easy::HighlightLines::new(syntax, theme);
                    prev_syntax = syntax as *const _;
                }
            }

            if display_to_be_processed_file {
                progress.info(format!("{path:?}"))
            }
            let out = chunk::process(
                &content,
                &mut img,
                |line| highlighter.highlight_line(line, &ss),
                chunk::Context {
                    column_width,
                    line_height,
                    total_line_count,
                    highlight_truncated_lines,
                    line_num,
                    lines_per_column,
                    fg_color,
                    bg_color,
                    file_index,
                    color_modulation,
                },
            )?;
            longest_line_chars = out.longest_line_in_chars.max(longest_line_chars);
            line_num += num_content_lines as u32;
            line_progress.inc_by(num_content_lines);
            background = out.background;
        }

        (line_num, longest_line_chars, background)
    } else {
        // multi-threaded rendering overview:
        //
        // Spawns threadpool and each file to be renered is sent to a thread as a message via a flume channel.
        // Upon recieving a message, a thread renders the entire file to an image of one column width.
        // and then returns that image to this main thread via a flume channel, to be stitched together
        // into one large image. The ordering of files rendered in the final image is remembered and
        // independant of thread rendering order.

        let mut line_num: u32 = 0;
        let mut longest_line_chars = 0;
        let mut background = None;
        std::thread::scope(|scope| -> anyhow::Result<()> {
            let (tx, rx) = flume::bounded::<(_, String, _, _, _)>(content.len());
            let (ttx, trx) = flume::unbounded();
            for tid in 0..threads {
                scope.spawn({
                    let rx = rx.clone();
                    let ttx = ttx.clone();
                    let ss = &ss;
                    let mut progress = line_progress.add_child(format!("Thread {tid}"));
                    move || -> anyhow::Result<()> {
                        let mut prev_syntax = ss.find_syntax_plain_text() as *const _;
                        let mut highlighter =
                            syntect::easy::HighlightLines::new(ss.find_syntax_plain_text(), theme);
                        for (path, content, num_content_lines, lines_so_far, file_index) in rx {
                            if !plain {
                                let syntax = ss
                                    .find_syntax_for_file(&path)?
                                    .unwrap_or_else(|| ss.find_syntax_plain_text());
                                if syntax as *const _ != prev_syntax {
                                    highlighter = syntect::easy::HighlightLines::new(syntax, theme);
                                    prev_syntax = syntax as *const _;
                                }
                            }

                            // create an image that fits one column
                            let mut img =
                                RgbImage::new(column_width, num_content_lines as u32 * line_height);

                            if display_to_be_processed_file {
                                progress.info(format!("{path:?}"))
                            }
                            let out = chunk::process(
                                &content,
                                &mut img,
                                |line| highlighter.highlight_line(line, ss),
                                chunk::Context {
                                    column_width,
                                    line_height,
                                    total_line_count,
                                    highlight_truncated_lines,
                                    line_num: 0,
                                    lines_per_column: total_line_count,
                                    fg_color,
                                    bg_color,
                                    file_index,
                                    color_modulation,
                                },
                            )?;
                            ttx.send((img, out, num_content_lines, lines_so_far))?;
                        }
                        Ok(())
                    }
                });
            }
            drop((rx, ttx));
            let mut lines_so_far = 0u32;
            for (file_index, ((path, content), num_content_lines)) in
                content.into_iter().enumerate()
            {
                tx.send((path, content, num_content_lines, lines_so_far, file_index))?;
                lines_so_far += num_content_lines as u32;
            }
            drop(tx);

            // for each file image that was rendered by a thread.
            for (sub_img, out, num_content_lines, lines_so_far) in trx {
                longest_line_chars = out.longest_line_in_chars.max(longest_line_chars);
                background = out.background;

                let calc_offsets = |line_num: u32| {
                    let actual_line = line_num % total_line_count;
                    calc_offsets(actual_line, lines_per_column, column_width, line_height)
                };

                // transfer pixels from sub_img to img. Where sub_img is a 1 column wide
                // image of one file. And img is our multi-column wide final output image.
                for line in 0..num_content_lines as u32 {
                    let (x_offset, line_y) = calc_offsets(lines_so_far + line);
                    for x in 0..column_width {
                        for height in 0..line_height {
                            let pix = sub_img.get_pixel(x, line * line_height + height);
                            img.put_pixel(x_offset + x, line_y + height, *pix);
                        }
                    }
                }

                line_progress.inc_by(num_content_lines);
                line_num += num_content_lines as u32;
                progress.inc();
                if should_interrupt.load(Ordering::Relaxed) {
                    bail!("Cancelled by user")
                }
            }
            Ok(())
        })?;
        (line_num, longest_line_chars, background)
    };

    // fill in any empty bottom right corner, with background color
    while line_num < lines_per_column * required_columns {
        let (cur_column_x_offset, cur_y) =
            calc_offsets(line_num, lines_per_column, column_width, line_height);
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
