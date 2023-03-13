use crate::render::chunk::calc_offsets;
use crate::render::Cache;
use crate::render::Dimension;
use crate::render::{chunk, Options};
use crate::DirContents;
use crate::FILENAME_LINE_COUNT;
use anyhow::{bail, Context};
use image::{ImageBuffer, Pixel, Rgb, RgbImage};
use memmap2::MmapMut;
use prodash::Progress;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

/// Render the given files to an image. Using the given syntax, theme and render options.
pub fn render(
    dir_content: &DirContents,
    mut progress: impl Progress,
    should_interrupt: &AtomicBool,
    ss: &SyntaxSet,
    ts: &ThemeSet,
    Options {
        column_width,
        line_height,
        readable,
        show_filenames,
        target_aspect_ratio,
        threads,
        fg_color,
        bg_color,
        highlight_truncated_lines,
        display_to_be_processed_file,
        theme,
        force_full_columns,
        plain,
        ignore_files_without_syntax,
        color_modulation,
        tab_spaces,
    }: Options,
) -> anyhow::Result<ImageBuffer<Rgb<u8>, MmapMut>> {
    // unused for now
    // could be used to make a "rolling code" animation
    let start = std::time::Instant::now();

    let mut line_height = line_height;
    let mut char_width = 1;
    if readable {
        line_height = 16;
        char_width = 8;
    }
    let line_height = line_height;
    let char_width = char_width;

    //> read files (for /n counting)
    let (content, total_line_count, num_ignored) = {
        let mut out = Vec::with_capacity(dir_content.children_content.len());
        let mut lines = 0;
        let mut num_ignored = 0;
        let mut lines_so_far = 0u32;
        for (path, content) in &dir_content.children_content {
            let num_content_lines = content.lines().count();
            lines += num_content_lines;
            if ignore_files_without_syntax && ss.find_syntax_for_file(path)?.is_none() {
                lines -= num_content_lines;
                num_ignored += 1;
            } else {
                out.push(((path, content), num_content_lines, lines_so_far));
                lines_so_far += num_content_lines as u32;
                if show_filenames {
                    lines_so_far += FILENAME_LINE_COUNT;
                }
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

    // add lines if displaying filenames.
    let mut total_line_count = total_line_count;
    if show_filenames {
        total_line_count += content.len() as u32 * FILENAME_LINE_COUNT;
    }
    // re-make immutable
    let total_line_count = total_line_count;

    // determine number and height of columns closest to desired aspect ratio
    let Dimension {
        imgx,
        imgy,
        lines_per_column,
        required_columns,
    } = crate::render::dimension::compute(
        target_aspect_ratio,
        column_width * char_width,
        total_line_count,
        line_height,
        force_full_columns,
        progress.add_child("determine dimensions"),
    )?;

    let num_pixels = {
        let channel_count = Rgb::<u8>::CHANNEL_COUNT;
        let num_pixels = imgx as usize * imgy as usize * channel_count as usize;
        progress.info(format!(
            "Image dimensions: {imgx} x {imgy} x {channel_count} [x * y * channels] ({} in virtual memory)",
            bytesize::ByteSize(num_pixels as u64 ),
        ));
        num_pixels
    };

    let mut img = ImageBuffer::<Rgb<u8>, _>::from_raw(imgx, imgy, MmapMut::map_anon(num_pixels)?)
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
    let mut cache = Cache::new_with_plain_highlighter(
        ss,
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

    let threads = (threads == 0)
        .then(num_cpus::get)
        .unwrap_or(threads)
        .clamp(1, num_cpus::get());
    let (mut line_num, longest_line_chars, background) = if threads < 2 {
        // single-threaded rendering overview:
        //
        // Sqeuentially iterates over the contents of each file to render.
        // Each time rendering it's text to the correct location in the final image.
        // No intermediate images are used like in multi-threaded rendering.

        let mut line_num: u32 = 0;
        let mut longest_line_chars = 0;
        let mut background = None;
        let mut highlighter = cache.new_plain_highlighter();
        for (file_index, ((path, content), num_content_lines, _lines_so_far)) in
            content.into_iter().enumerate()
        {
            progress.inc();
            if should_interrupt.load(Ordering::Relaxed) {
                bail!("Cancelled by user")
            }
            if !plain {
                if let Some(hl) = cache.highlighter_for_file_name(path)? {
                    highlighter = hl;
                }
            }

            let relative_path = path.strip_prefix(&dir_content.parent_dir).unwrap();
            if display_to_be_processed_file {
                progress.info(format!("{relative_path:?}"))
            }
            let out = chunk::process(
                &relative_path,
                content,
                &mut img,
                |line| highlighter.highlight_line(line, ss),
                chunk::Context {
                    column_width,
                    line_height,
                    char_width,
                    total_line_count,
                    highlight_truncated_lines,
                    line_num,
                    lines_per_column,
                    fg_color,
                    bg_color,
                    file_index,
                    color_modulation,
                    tab_spaces,
                    readable,
                    show_filenames,
                },
            )?;
            longest_line_chars = out.longest_line_in_chars.max(longest_line_chars);
            line_num += num_content_lines as u32;
            if show_filenames {
                line_num += FILENAME_LINE_COUNT
            };
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
        let file_index = AtomicUsize::default();
        std::thread::scope(|scope| -> anyhow::Result<()> {
            let (ttx, trx) = flume::unbounded();
            for tid in 0..threads {
                scope.spawn({
                    let ttx = ttx.clone();
                    let file_index = &file_index;
                    let ss = &ss;
                    let content = &content;
                    let mut state = cache.clone();
                    let mut progress = line_progress.add_child(format!("Thread {tid}"));
                    move || -> anyhow::Result<()> {
                        let mut highlighter = state.new_plain_highlighter();
                        while let Ok(file_index) =
                            file_index.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| {
                                (x < content.len()).then_some(x + 1)
                            })
                        {
                            let ((path, content), num_content_lines, lines_so_far) =
                                &content[file_index];
                            if !plain {
                                if let Some(hl) = state.highlighter_for_file_name(path)? {
                                    highlighter = hl;
                                }
                            }

                            let img_height = if show_filenames {
                                (*num_content_lines as u32 * line_height)
                                    + line_height * FILENAME_LINE_COUNT
                            } else {
                                *num_content_lines as u32 * line_height
                            };

                            // create an image that fits one column
                            let mut img = RgbImage::new(column_width * char_width, img_height);

                            let relative_path = path.strip_prefix(&dir_content.parent_dir).unwrap();
                            if display_to_be_processed_file {
                                progress.info(format!("{relative_path:?}"))
                            }
                            let out = chunk::process(
                                &relative_path,
                                content,
                                &mut img,
                                |line| highlighter.highlight_line(line, ss),
                                chunk::Context {
                                    column_width,
                                    line_height,
                                    char_width,
                                    total_line_count,
                                    highlight_truncated_lines,
                                    line_num: 0,
                                    lines_per_column: total_line_count,
                                    fg_color,
                                    bg_color,
                                    file_index,
                                    color_modulation,
                                    tab_spaces,
                                    readable,
                                    show_filenames,
                                },
                            )?;
                            ttx.send((img, out, *num_content_lines, *lines_so_far))?;
                        }
                        Ok(())
                    }
                });
            }
            drop(ttx);

            // for each file image that was rendered by a thread.
            for (sub_img, out, num_content_lines, lines_so_far) in trx {
                longest_line_chars = out.longest_line_in_chars.max(longest_line_chars);
                background = out.background;

                let calc_offsets = |line_num: u32| {
                    let actual_line = line_num % total_line_count;
                    calc_offsets(actual_line, lines_per_column, column_width, line_height)
                };

                let mut lines_in_sub_img = num_content_lines as u32;
                if show_filenames {
                    lines_in_sub_img += FILENAME_LINE_COUNT;
                }
                // transfer pixels from sub_img to img. Where sub_img is a 1 column wide
                // image of one file. And img is our multi-column wide final output image.
                for line in 0..lines_in_sub_img {
                    let (x_offset, line_y) = calc_offsets(lines_so_far + line);
                    for x in 0..column_width * char_width {
                        for height in 0..line_height {
                            let pix = sub_img.get_pixel(x, line * line_height + height);
                            img.put_pixel(x_offset * char_width + x, line_y + height, *pix);
                        }
                    }
                }

                line_progress.inc_by(num_content_lines);
                line_num += num_content_lines as u32;
                if show_filenames {
                    line_num += FILENAME_LINE_COUNT
                };
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

        for cur_line_x in 0..column_width * char_width {
            for y_pos in cur_y..cur_y + line_height {
                img.put_pixel(
                    cur_column_x_offset * char_width + cur_line_x,
                    y_pos,
                    background,
                );
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
