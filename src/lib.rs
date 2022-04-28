#[cfg(test)]
mod tests {
    use crate::renderer;

    #[test]
    fn renders_self() {
        let paths = renderer::get_unicode_files_in_dir("./src/");
        let img = renderer::render(&paths, 100, 16.0 / 9.0, true, true);
        if img == None {
            panic!();
        }
    }
}

pub mod renderer {

    use glob::glob;
    use image::{ImageBuffer, Rgb, RgbImage};
    use std::fs;
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::path::PathBuf;
    use syntect::easy::HighlightFile;
    use syntect::highlighting::{Style, ThemeSet};
    use syntect::parsing::SyntaxSet;

    pub fn render(
        paths: &[PathBuf],
        column_width: u32,
        target_aspect_ratio: f64,
        force_full_columns: bool,
        print_progress: bool,
    ) -> Option<ImageBuffer<Rgb<u8>, Vec<u8>>> {
        // unused for now
        // could be used to make a "rolling code" animation
        let line_offset = 0;

        //> read files (for /n counting)
            let mut line_count = 0;
            for path in paths {
                let filename = path;
                // Open the file in read-only mode (ignoring errors).
                let file = File::open(filename).unwrap();
                let reader = BufReader::new(file);

                let line_count_usize = reader.lines().count();
                line_count += line_count_usize as u32;
            }
            // re-make immutable
            let line_count = line_count;
        //<

        if line_count == 0 {
            return None;
        }

        //> determine image dimensions based on num of lines and contraints

            //> initialize variables
                let mut last_checked_aspect_ratio: f64 = f64::MAX;
                let mut column_line_limit = 1;
                let mut last_column_line_limit = column_line_limit;
                let mut required_columns;
                let mut cur_aspect_ratio: f64 =
                    column_width as f64 * line_count as f64 / (column_line_limit as f64 * 2.0);

            //<> determine maximum aspect ratios
                let tallest_aspect_ratio = column_width as f64 / line_count as f64 * 2.0;
                let widest_aspect_ratio = line_count as f64 * column_width as f64 / 2.0;
            //<

            if target_aspect_ratio <= tallest_aspect_ratio {
                //> use tallest possible aspect ratio
                    column_line_limit = line_count;
                    required_columns = 1;
                //<
            } else if target_aspect_ratio >= widest_aspect_ratio {
                //> use widest possible aspect ratio
                    column_line_limit = 1;
                    required_columns = line_count;
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
                            required_columns = line_count / column_line_limit;
                            if line_count % column_line_limit != 0 {
                                required_columns += 1;
                            }
                        //<

                        let last_required_columns = required_columns;

                        // find next full column aspect ratio
                        while required_columns == last_required_columns {
                            column_line_limit += 1;

                            //> determine required number of columns
                                required_columns = line_count / column_line_limit;
                                if line_count % column_line_limit != 0 {
                                    required_columns += 1;
                                }
                            //<
                        }
                    } else {
                        //> generate new aspect ratio
                            column_line_limit += 1;

                            //> determine required number of columns
                                required_columns = line_count / column_line_limit;
                                if line_count % column_line_limit != 0 {
                                    required_columns += 1;
                                }
                            //<
                        //<
                    }

                    cur_aspect_ratio = required_columns as f64 * column_width as f64
                        / (column_line_limit as f64 * 2.0);
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
                        required_columns = line_count / column_line_limit;
                        if line_count % column_line_limit != 0 {
                            required_columns += 1;
                        }
                    //<
                //<
            }

            //> remake immutable
                let required_columns = required_columns;
                let column_line_limit = column_line_limit;
            //<

        //<> initialize image
            //> determine x
                let imgx: u32 = required_columns * column_width;

            //<> determine y
                let imgy: u32 = if line_count < column_line_limit {
                    line_count * 2
                } else {
                    column_line_limit * 2
                };
            //<

            // Create a new ImgBuf with width: imgx and height: imgy
            let mut imgbuf = RgbImage::new(imgx, imgy);

        //<> initialize rendering vars
            let mut cur_line_x = 0;
            let mut line = String::new();
            let mut line_num: u32 = 0;
            let mut background = Rgb([0, 0, 0]);

        //<> vars for rendering a progress bar
            let tq = tqdm_rs::Tqdm::manual(paths.len());
            let mut path_num = 1;
        //<

        // render all lines onto image
        for path in paths {
            if print_progress {
                println!("{}", path.display());
                tq.update(path_num);
                path_num += 1;
            }

            //> initialize highlighting themes
                let ss = SyntaxSet::load_defaults_newlines();
                let ts = ThemeSet::load_defaults();
                let mut highlighter =
                    HighlightFile::new(path, &ss, &ts.themes["Solarized (dark)"]).unwrap();
            //<

            while highlighter.reader.read_line(&mut line).unwrap() > 0 {
                {
                    //> get position of current line
                        //> y
                            let actual_line = (line_num + line_offset) % line_count;
                            let cur_y = (actual_line % column_line_limit) * 2;
                        //<> x
                            let cur_column_x_offset = (actual_line / column_line_limit) * column_width;
                        //<
                    //<

                    let regions: Vec<(Style, &str)> =
                        highlighter.highlight_lines.highlight(&line, &ss);

                    background = Rgb([
                        regions[0].0.background.r,
                        regions[0].0.background.g,
                        regions[0].0.background.b,
                    ]);

                    for region in regions {
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
                                    imgbuf.put_pixel(
                                        cur_column_x_offset + cur_line_x,
                                        cur_y,
                                        background,
                                    );
                                    imgbuf.put_pixel(
                                        cur_column_x_offset + cur_line_x,
                                        cur_y + 1,
                                        background,
                                    );

                                    cur_line_x += 1;
                                } else if chr == '\t' {
                                    // specifies how many spaces a tab should be rendered as
                                    let tab_spaces = 4;

                                    let spaces_to_add = tab_spaces - (cur_line_x % tab_spaces);

                                    for x in cur_line_x..cur_line_x + spaces_to_add {
                                        if x >= column_width {
                                            break;
                                        }

                                        imgbuf.put_pixel(
                                            cur_column_x_offset + cur_line_x,
                                            cur_y,
                                            background,
                                        );
                                        imgbuf.put_pixel(
                                            cur_column_x_offset + cur_line_x,
                                            cur_y + 1,
                                            background,
                                        );

                                        cur_line_x += 1;
                                    }
                                } else {
                                    imgbuf.put_pixel(
                                        cur_column_x_offset + cur_line_x,
                                        cur_y,
                                        char_color,
                                    );
                                    imgbuf.put_pixel(
                                        cur_column_x_offset + cur_line_x,
                                        cur_y + 1,
                                        char_color,
                                    );

                                    cur_line_x += 1;
                                }
                            //<
                        }
                    }

                    while cur_line_x < column_width {
                        imgbuf.put_pixel(cur_column_x_offset + cur_line_x, cur_y, background);
                        imgbuf.put_pixel(cur_column_x_offset + cur_line_x, cur_y + 1, background);

                        cur_line_x += 1;
                    }

                    cur_line_x = 0;
                    line_num += 1;
                } // until NLL this scope is needed so we can clear the buffer after
                line.clear(); // read_line appends so we need to clear between lines
            }
        }

        //> fill in any empty bottom right corner, with background color
            while line_num < column_line_limit * required_columns {
                //> get position of current line
                    //> y
                        // let actual_line = (line_num + line_offset) % line_count;
                        let cur_y = (line_num % column_line_limit) * 2;
                    //<> x
                        let cur_column_x_offset = (line_num / column_line_limit) * column_width;
                    //<

                //<> fill line with background color
                    for cur_line_x in 0..column_width {
                        imgbuf.put_pixel(cur_column_x_offset + cur_line_x, cur_y, background);
                        imgbuf.put_pixel(cur_column_x_offset + cur_line_x, cur_y + 1, background);
                    }
                //<
                line_num += 1;
            }
        //<

        if print_progress {
            println!("===== Finished Render Stats =====");
            println!("line_count: {}", line_count);
            println!(
                "Aspect ratio is {} off from target",
                (last_checked_aspect_ratio - target_aspect_ratio).abs()
            );
            println!("=================================");
        }

        Some(imgbuf)
    }

    pub fn get_unicode_files_in_dir(path: &str) -> Vec<PathBuf> {
        //> get list of all files in ./input/ using glob
            let mut paths = Vec::new();

            let file_delimiter = "";
            let search_params = String::from(path) + "**/*" + file_delimiter;

            for entry in glob(&search_params).expect("Failed to read glob pattern") {
                match entry {
                    Ok(path) => {
                        paths.push(path);
                    }
                    Err(e) => println!("{:?}", e),
                }
            }

        //<> filter out directories
            let paths = paths.into_iter().filter(|e| e.is_file());

        //<> filter out non unicode files
            let paths: Vec<PathBuf> = paths
                .into_iter()
                .filter(|e| fs::read_to_string(e).is_ok())
                .collect();
        //<

        paths
    }
}
