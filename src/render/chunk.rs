use crate::render::{BgColor, FgColor};
use crate::RenderType;
use bstr::ByteSlice;
use image::{ImageBuffer, Rgb};
use std::ops::{Deref, DerefMut};
use std::path::Path;
use syntect::highlighting::{Color, Style};
use unifont_bitmap::Unifont;

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
    pub char_width: u32,
    pub total_line_count: u32,
    pub line_num: u32,
    pub lines_per_column: u32,

    pub fg_color: FgColor,
    pub bg_color: BgColor,
    pub highlight_truncated_lines: bool,

    pub file_index: usize,
    pub color_modulation: f32,
    pub tab_spaces: u32,
    pub readable: bool,
    pub show_filenames: bool,
    pub line_nums: bool,
}

/// Return the `(x, y)` offsets to apply to the given line, to wrap columns of lines into the
/// target image.
pub fn calc_offsets(
    line_num: u32,
    lines_per_column: u32,
    column_width: u32,
    line_height: u32,
) -> (u32, u32) {
    (
        (line_num / lines_per_column) * column_width,
        (line_num % lines_per_column) * line_height,
    )
}

/// Ensures a string has a minimum number of characters in it by
/// appending spaces to the beginging of the string if its characters
/// are too few. This is used for formatting the spacing of line_nums.
fn ensure_width(str: String, width: u32) -> String {
    let spaces_to_add = width as usize - str.len();
    let mut spaces = String::new();
    for _ in 0..spaces_to_add {
        spaces.push(' ');
    }
    spaces + &str
}

/// Renders text onto an existing image.
///
/// Images of proper dimensions must be pre-allocated before calling this function.
/// This function can be used to render one file/string of text, to a single image,
/// or called repeatedly on different files/strings, and passed the same image, to
/// render different bodies of text in different areas of the same image.
pub fn process(
    filepath: &Path,
    content: &str,
    img: &mut RenderType,
    mut highlight: impl FnMut(&str) -> Result<Vec<(Style, &str)>, syntect::Error>,
    Context {
        column_width,
        line_height,
        char_width,
        total_line_count,
        highlight_truncated_lines,
        mut line_num,
        lines_per_column,
        fg_color,
        bg_color,
        file_index,
        color_modulation,
        tab_spaces,
        readable,
        show_filenames,
        line_nums,
    }: Context,
) -> anyhow::Result<Outcome> {
    let mut unifont = Unifont::open();

    let largest_line_num_width = if line_nums {
        format!("{}", content.lines().count()).len() + 1
    } else {
        // We don't need it for rendering.
        // So pass default value.
        0
    };

    let style = highlight(" ")?[0].0;
    let initial_forground_color = Rgb([style.foreground.r, style.foreground.g, style.foreground.b]);

    // write the filename
    if show_filenames {
        // get background color
        // let style = highlight(" ")?[0].0;
        // println!("style: {:#?}", style);
        let mut background = None::<Rgb<u8>>;
        let background =
            background.get_or_insert_with(|| bg_color.to_rgb(style, file_index, color_modulation));

        // figure out where in the image to write
        let actual_line = line_num % total_line_count;
        let (cur_column_x_offset, cur_y) = calc_offsets(
            actual_line,
            lines_per_column,
            column_width * char_width,
            line_height,
        );

        // write filename on image
        // let char_color = Rgb([255, 255, 255]);
        let mut cur_line_x = 0;
        for chr in filepath.to_str().unwrap().chars() {
            if readable {
                put_readable_char_in_image(
                    chr,
                    &mut unifont,
                    cur_column_x_offset + cur_line_x * char_width,
                    cur_y,
                    img,
                    &background,
                    &initial_forground_color,
                    &mut cur_line_x,
                );
            } else {
                // Fill the char space with a solid color.
                let img_x = cur_column_x_offset + cur_line_x;
                put_solid_char_in_image(
                    img_x,
                    cur_y,
                    img,
                    initial_forground_color,
                    line_height,
                    char_width,
                    &mut cur_line_x,
                );
            }
        }

        // Fill the rest of the line with the background color.
        if readable {
            while cur_line_x < column_width {
                put_readable_char_in_image(
                    ' ',
                    &mut unifont,
                    cur_column_x_offset + cur_line_x * char_width,
                    cur_y,
                    img,
                    background,
                    background,
                    &mut cur_line_x,
                );
            }
        } else {
            while cur_line_x < column_width * char_width {
                // Fill the char space with a solid color.
                let img_x = cur_column_x_offset + cur_line_x;
                put_solid_char_in_image(
                    img_x,
                    cur_y,
                    img,
                    *background,
                    line_height,
                    char_width,
                    &mut cur_line_x,
                );
            }
        }

        line_num += 1;
    }

    // render all lines in `content` to image
    let mut longest_line_in_chars = 0;
    let mut background = None::<Rgb<u8>>;
    for (file_line_num, line) in content.as_bytes().lines_with_terminator().enumerate() {
        // make file_line_num that of the file.
        let file_line_num = file_line_num + 1;

        let (line, truncated_line) = {
            let line = line.to_str().expect("UTF-8 was source");
            let mut num_chars = 0;
            let mut chars = line.chars();
            let bytes_till_char_limit: usize = chars
                .by_ref()
                .take(column_width as usize)
                .map(|c| {
                    num_chars += 1;
                    c.len_utf8()
                })
                .sum();
            num_chars += chars.count();
            longest_line_in_chars = longest_line_in_chars.max(num_chars);
            let possibly_truncated_line = (num_chars >= column_width as usize)
                .then(|| &line[..bytes_till_char_limit])
                .unwrap_or(line);
            (
                if highlight_truncated_lines {
                    possibly_truncated_line
                } else {
                    line
                },
                possibly_truncated_line,
            )
        };

        let actual_line = line_num % total_line_count;
        let (cur_column_x_offset, cur_y) = calc_offsets(
            actual_line,
            lines_per_column,
            column_width * char_width,
            line_height,
        );
        let storage;
        let array_storage;

        let regions: &[_] = if line.len() > 1024 * 16 {
            array_storage = [(default_bg_color(background), truncated_line)];
            &array_storage
        } else {
            storage = highlight(line)?;
            &storage
        };
        let background = background
            .get_or_insert_with(|| bg_color.to_rgb(regions[0].0, file_index, color_modulation));
        let mut cur_line_x = 0;

        // draw file_line_num for this line
        if line_nums {
            let line_num_string =
                ensure_width(format!("{}", file_line_num), largest_line_num_width as u32) + " ";

            let file_line_num_char_color = initial_forground_color;
            // let file_line_num_char_color = Rgb([255, 255, 255]);
            for chr in line_num_string.chars() {
                if readable {
                    put_readable_char_in_image(
                        chr,
                        &mut unifont,
                        cur_column_x_offset + cur_line_x * char_width,
                        cur_y,
                        img,
                        background,
                        &file_line_num_char_color,
                        &mut cur_line_x,
                    );
                } else {
                    let color = if chr == ' ' {
                        *background
                    } else {
                        file_line_num_char_color
                    };
                    // Fill the char space with a solid color.
                    let img_x = cur_column_x_offset + cur_line_x;
                    put_solid_char_in_image(
                        img_x,
                        cur_y,
                        img,
                        color,
                        line_height,
                        char_width,
                        &mut cur_line_x,
                    );
                }
            }
        }

        // Draw the line on the image.
        for (style, region) in regions {
            if cur_line_x >= column_width * char_width {
                break;
            }
            if region.is_empty() {
                continue;
            }

            for chr in region.chars() {
                if cur_line_x >= column_width * char_width {
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
                    if readable {
                        put_readable_char_in_image(
                            ' ',
                            &mut unifont,
                            cur_column_x_offset + cur_line_x * char_width,
                            cur_y,
                            img,
                            background,
                            &char_color,
                            &mut cur_line_x,
                        );
                    } else {
                        // Fill the char space with a solid color.
                        let img_x = cur_column_x_offset + cur_line_x;
                        put_solid_char_in_image(
                            img_x,
                            cur_y,
                            img,
                            *background,
                            line_height,
                            char_width,
                            &mut cur_line_x,
                        );
                    }
                } else if chr == '\t' {
                    let spaces_to_add = tab_spaces - (cur_line_x % tab_spaces);

                    for _ in 0..spaces_to_add {
                        if cur_line_x >= column_width * char_width {
                            break;
                        }

                        if readable {
                            put_readable_char_in_image(
                                ' ',
                                &mut unifont,
                                cur_column_x_offset + cur_line_x * char_width,
                                cur_y,
                                img,
                                background,
                                &char_color,
                                &mut cur_line_x,
                            );
                        } else {
                            // Fill the char space with a solid color.
                            let img_x = cur_column_x_offset + cur_line_x;
                            put_solid_char_in_image(
                                img_x,
                                cur_y,
                                img,
                                *background,
                                line_height,
                                char_width,
                                &mut cur_line_x,
                            );
                        }
                    }
                } else if readable {
                    put_readable_char_in_image(
                        chr,
                        &mut unifont,
                        cur_column_x_offset + cur_line_x * char_width,
                        cur_y,
                        img,
                        background,
                        &char_color,
                        &mut cur_line_x,
                    );
                } else {
                    // Fill the char space with a solid color.
                    let img_x = cur_column_x_offset + cur_line_x;
                    put_solid_char_in_image(
                        img_x,
                        cur_y,
                        img,
                        char_color,
                        line_height,
                        char_width,
                        &mut cur_line_x,
                    );
                }
            }
        }

        // Fill the rest of the line with the background color.
        if readable {
            while cur_line_x < column_width {
                put_readable_char_in_image(
                    ' ',
                    &mut unifont,
                    cur_column_x_offset + cur_line_x * char_width,
                    cur_y,
                    img,
                    background,
                    background,
                    &mut cur_line_x,
                );
            }
        } else {
            while cur_line_x < column_width * char_width {
                // Fill the char space with a solid color.
                let img_x = cur_column_x_offset + cur_line_x;
                put_solid_char_in_image(
                    img_x,
                    cur_y,
                    img,
                    *background,
                    line_height,
                    char_width,
                    &mut cur_line_x,
                );
            }
        }

        line_num += 1;
    }

    Ok(Outcome {
        longest_line_in_chars,
        background,
    })
}

fn put_readable_char_in_image(
    chr: char,
    unifont: &mut Unifont,
    img_x: u32,
    img_y: u32,
    img: &mut RenderType,
    background_color: &Rgb<u8>,
    text_color: &Rgb<u8>,
    cur_line_x: &mut u32,
) {
    let bitmap = unifont.load_bitmap(chr.into());

    // get bitmap dimensions
    let char_height = 16;
    // let standard_char_width = 8;
    let char_width = if bitmap.is_wide() { 16 } else { 8 };

    // add bitmap to image
    for y in 0..char_height as usize {
        for x in 0..char_width {
            let pixel_x = img_x + x;
            let pixel_y = img_y + y as u32;

            // get pixel from bitmap
            let should_pixel = if bitmap.is_wide() {
                bitmap.get_bytes()[y * 2 + x as usize / 8] & (1 << (7 - x % 8)) != 0
            } else {
                bitmap.get_bytes()[y] & (1 << (7 - x)) != 0
            };

            // skip pixel if out of bounds
            match img {
                RenderType::MmapImage(img) => {
                    if pixel_x >= img.width() || pixel_y >= img.height() {
                        continue;
                    }
                }
                RenderType::Image(img) => {
                    if pixel_x >= img.width() || pixel_y >= img.height() {
                        continue;
                    }
                }
                RenderType::TileCache(_) => {}
            }

            // set pixel in image
            if should_pixel {
                img.put_pixel(
                    pixel_x.try_into().unwrap(),
                    pixel_y.try_into().unwrap(),
                    *text_color,
                );
            } else {
                img.put_pixel(
                    pixel_x.try_into().unwrap(),
                    pixel_y.try_into().unwrap(),
                    *background_color,
                );
            }
        }
    }

    if bitmap.is_wide() {
        *cur_line_x += 2;
    } else {
        *cur_line_x += 1;
    }
}

/// Fill the char space with a solid color.
fn put_solid_char_in_image(
    img_x: u32,
    img_y: u32,
    img: &mut RenderType,
    color: Rgb<u8>,
    line_height: u32,
    char_width: u32,
    cur_line_x: &mut u32,
) {
    // println!("placeing char");
    // Fill the char space with a solid color.
    for y_pos in img_y..img_y + line_height {
        // println!("placing y");
        for x_pos in img_x..img_x + char_width {
            // println!("placing x");
            img.put_pixel(x_pos.try_into().unwrap(), y_pos.try_into().unwrap(), color);
        }
    }
    *cur_line_x += char_width;
}

fn default_bg_color(background: Option<Rgb<u8>>) -> Style {
    Style {
        foreground: Color {
            r: 200,
            g: 200,
            b: 200,
            a: u8::MAX,
        },
        background: background
            .map(|c| Color {
                r: c.0[0],
                g: c.0[1],
                b: c.0[2],
                a: u8::MAX,
            })
            .unwrap_or(Color::BLACK),
        font_style: Default::default(),
    }
}
