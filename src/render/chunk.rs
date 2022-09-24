use crate::render::{BgColor, FgColor};
use bstr::ByteSlice;
use image::{ImageBuffer, Rgb};
use std::ops::{Deref, DerefMut};
use syntect::highlighting::{Color, Style};

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
    pub highlight_truncated_lines: bool,

    pub file_index: usize,
    pub color_modulation: f32,
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

pub fn process<C>(
    content: &str,
    img: &mut ImageBuffer<Rgb<u8>, C>,
    mut highlight: impl FnMut(&str) -> Result<Vec<(Style, &str)>, syntect::Error>,
    Context {
        column_width,
        line_height,
        total_line_count,
        highlight_truncated_lines,
        mut line_num,
        lines_per_column,
        fg_color,
        bg_color,
        file_index,
        color_modulation,
    }: Context,
) -> anyhow::Result<Outcome>
where
    C: Deref<Target = [u8]>,
    C: DerefMut,
{
    let mut longest_line_in_chars = 0;
    let mut background = None::<Rgb<u8>>;
    for line in content.as_bytes().lines_with_terminator() {
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
        let (cur_column_x_offset, cur_y) =
            calc_offsets(actual_line, lines_per_column, column_width, line_height);
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

    Ok(Outcome {
        longest_line_in_chars,
        background,
    })
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
