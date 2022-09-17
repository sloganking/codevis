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
#[derive(clap::ValueEnum, Clone, Copy, Debug, Eq, PartialEq)]
pub enum BgColor {
    /// Use the style of the syntax to color the background pixel.
    Style,
    /// Use the style of the syntax to color the background pixel and modulate it in an even-odd pattern
    /// to make file borders visible.
    StyleCheckerboardDarken,
    /// Use the style of the syntax to color the background pixel and modulate it in an even-odd pattern
    /// to make file borders visible.
    StyleCheckerboardBrighten,
    /// The purple color of the Helix Editor.
    HelixEditor,
}

impl BgColor {
    pub fn to_rgb(&self, style: Style, file_index: usize, color_modulation: f32) -> Rgb<u8> {
        match self {
            BgColor::Style => Rgb([style.background.r, style.background.g, style.background.b]),
            BgColor::HelixEditor => Rgb([59, 34, 76]),
            BgColor::StyleCheckerboardDarken | BgColor::StyleCheckerboardBrighten => {
                let m = if self == &BgColor::StyleCheckerboardBrighten {
                    (file_index % 2 == 0)
                        .then(|| 1.0 + color_modulation)
                        .unwrap_or(1.0)
                } else {
                    (file_index % 2 == 0)
                        .then(|| 1.0)
                        .unwrap_or_else(|| (1.0_f32 - color_modulation).max(0.0))
                };
                Rgb([
                    (style.background.r as f32 * m).min(255.0) as u8,
                    (style.background.g as f32 * m).min(255.0) as u8,
                    (style.background.b as f32 * m).min(255.0) as u8,
                ])
            }
        }
    }
}

/// Configure how to render an image.
#[derive(Debug, Copy, Clone)]
pub struct Options<'a> {
    pub column_width: u32,
    pub line_height: u32,
    pub target_aspect_ratio: f64,

    pub threads: usize,
    pub highlight_truncated_lines: bool,

    pub fg_color: FgColor,
    pub bg_color: BgColor,
    pub theme: &'a str,

    pub force_full_columns: bool,
    pub ignore_files_without_syntax: bool,
    pub plain: bool,
    pub display_to_be_processed_file: bool,
    pub color_modulation: f32,
}

mod highlight;
use highlight::Cache;

pub(crate) mod function;

mod chunk;

mod dimension;
use dimension::Dimension;
