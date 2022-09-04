use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
pub struct Args {
    /// If true, files that would be rendered white due to lack of syntax are skipped.
    #[clap(long, help_heading = "INPUT")]
    pub ignore_files_without_syntax: bool,

    /// Assure columns are never empty and continuously filled.
    #[clap(long, default_value_t = true, help_heading = "IMAGE")]
    pub force_full_columns: bool,

    /// The width of one column of lines of code in pixels.
    ///
    /// The maximum length of a line.
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

    /// The path to which to write the output png file
    #[clap(
        long,
        short = 'o',
        default_value = "output.png",
        help_heading = "OUTPUT"
    )]
    pub output_path: PathBuf,
}
