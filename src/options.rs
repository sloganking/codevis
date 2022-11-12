use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
#[clap(version)]
pub struct Args {
    /// The directory to read UTF-8 encoded text files from.
    #[clap(long, short = 'i', help_heading = "INPUT")]
    pub input_dir: PathBuf,

    /// An extension to ignore, like `md` for markdown files.
    /// You can add multiple extensions by seperating them with commas like so `--ignore_extension rs,lock`.
    #[clap(long, help_heading = "INPUT", value_delimiter = ',')]
    pub ignore_extension: Vec<OsString>,

    /// An extension to render, like `md` for markdown files. All other extensions will be ignored.
    /// You can add multiple extensions by seperating them with commas like so `--whitelist_extension rs,lock`.
    #[clap(
        long,
        conflicts_with("ignore_extension"),
        help_heading = "INPUT",
        value_delimiter = ','
    )]
    pub whitelist_extension: Vec<OsString>,

    /// If true, files that would be rendered white due to lack of syntax are skipped.
    #[clap(long, help_heading = "INPUT")]
    pub ignore_files_without_syntax: bool,

    /// The number of threads to use for rendering.
    ///
    /// '0' is equivalent to using all logical cores, this is also the default.
    #[clap(long, short = 't', default_value_t = num_cpus::get(), help_heading = "PERFORMANCE")]
    pub threads: usize,

    /// If true, highlighting will be performed on lines truncated to the `--column-width-pixels`, which is faster
    /// but may lock up syntax highlighting.
    ///
    /// It may also affect the looks.
    /// This is particularly interesting in conjunction with `--plain`, which will never lock up.
    #[clap(long, help_heading = "PERFORMANCE")]
    pub highlight_truncated_lines: bool,

    /// Only use plain text file syntax highlighting. It's fastest and won't lock up.
    #[clap(long, conflicts_with("theme"), help_heading = "PERFORMANCE")]
    pub force_plain_syntax: bool,

    /// When a file looks up, use this to see which file is about to be highlighted.
    #[clap(long, help_heading = "MONITORING")]
    pub display_to_be_processed_file: bool,

    /// Allow the last column to be partially empty, with the tradeoff
    /// of the output image being closer to desired aspect ratio.
    #[clap(long, help_heading = "IMAGE")]
    pub dont_force_full_columns: bool,

    /// The width of one column in pixels, with each character being a pixel wide.
    ///
    /// Lines longer than that will be truncated.
    #[clap(long, default_value_t = 100, help_heading = "IMAGE")]
    pub column_width_pixels: u32,

    /// The height of a line in pixels,
    #[clap(long, default_value_t = 2, help_heading = "IMAGE")]
    pub line_height_pixels: u32,

    /// The width side of the desired image aspect.
    #[clap(long, default_value_t = 16.0, help_heading = "IMAGE")]
    pub aspect_width: f64,

    /// The height side of the desired image aspect.
    #[clap(long, default_value_t = 9.0, help_heading = "IMAGE")]
    pub aspect_height: f64,

    /// The themes to use for rendering. Use `foo` to see a list of possible values.
    ///
    /// If multiple are specified, the output file name will be adjusted to match the theme accordingly.
    /// You can add multiple themes by seperating them with commas like so `--theme "Solarized (dark)","Solarized (light)"`.
    #[clap(long, default_values = &["Solarized (dark)"], help_heading = "COLORS", value_delimiter = ',')]
    pub theme: Vec<String>,

    /// Render the input with all available themes, one after another.
    #[clap(
        long,
        help_heading = "COLORS",
        conflicts_with("theme"),
        conflicts_with("force_plain_syntax")
    )]
    pub all_themes: bool,

    /// The way foreground pixels are colored.
    #[clap(value_enum, long, default_value_t = codevis::render::FgColor::StyleAsciiBrightness, help_heading = "COLORS")]
    pub fg_pixel_color: codevis::render::FgColor,

    /// The way background pixels are colored.
    #[clap(value_enum, long, default_value_t = codevis::render::BgColor::Style, help_heading = "COLORS")]
    pub bg_pixel_color: codevis::render::BgColor,

    /// The difference in brightness that certain background color styles may have at most.
    #[clap(long, default_value_t = 0.3, help_heading = "COLORS")]
    pub color_modulation: f32,

    /// Open the output image with the standard image viewer.
    #[clap(long, help_heading = "OUTPUT")]
    pub open: bool,

    /// The path to which to write the output png file
    #[clap(
        long,
        short = 'o',
        default_value = "output.png",
        help_heading = "OUTPUT"
    )]
    pub output_path: PathBuf,
}
