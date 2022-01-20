use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use image::{RgbImage, Rgb};

use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style};
use syntect::easy::HighlightFile;

use syntect;
use std::fs;

fn main() {

    // get list of valid files
        // get list of files in ./input/ using glob
            use glob::glob;

            let mut paths = Vec::new();

            let file_delimiter = "";
            let search_params = String::from("./input/**/*") + file_delimiter;

            for entry in glob(&search_params).expect("Failed to read glob pattern") {
                match entry {
                    Ok(path) => {
                        paths.push(path);
                    },
                    Err(e) => println!("{:?}", e),
                }
            }

        // filter out directories
            let paths: Vec<PathBuf> = paths.into_iter().filter(|e| e.is_file()).collect();

        // filter out non unicode files
            let paths: Vec<PathBuf> = paths.into_iter().filter(|e| {
                match fs::read_to_string(e){
                    Ok(_) => true,
                    Err(_) => false,
                }
            }).collect();

    // read files (for /n counting)
        let mut line_count = 0;
        for path in &paths{
            let filename = path;
            // Open the file in read-only mode (ignoring errors).
            let file = File::open(filename).unwrap();
            let reader = BufReader::new(file);

            let line_count_usize = reader.lines().count();
            line_count += line_count_usize as u32;
        }
        // re-make immutable
        let line_count = line_count;

        println!("line_count: {}",line_count);

    // determine image dimensions based on num of lines and contraints

        // this is a constraint
            let target_aspect_ratio: f64 = 16.0 / 9.0;
            // let target_aspect_ratio: f64 = 1284.0 / 2778.0; // iphone
            let column_width = 100;

        let mut last_checked_aspect_ratio: f64 = f64::MAX;
        let mut column_line_limit = 1;
        let mut required_columns = 1;
        let mut cur_aspect_ratio: f64 = column_width as f64 * line_count as f64 / (column_line_limit as f64 * 2.0);

        if target_aspect_ratio == 0.0{
            column_line_limit = line_count;
            required_columns = 1;
        }else{

            while (last_checked_aspect_ratio - target_aspect_ratio).abs() > (cur_aspect_ratio - target_aspect_ratio).abs(){

                // remember current aspect ratio
                    last_checked_aspect_ratio = cur_aspect_ratio;

                // generate new aspect ratio
                    column_line_limit += 1;

                    // determine required number of columns
                        required_columns = line_count / column_line_limit;
                        if line_count % column_line_limit != 0{
                            required_columns = required_columns + 1;
                        }

                    cur_aspect_ratio = required_columns as f64 * column_width as f64 / (column_line_limit as f64 * 2.0);
            
            }

            // previous while loop would never have been entered if (column_line_limit == 1)
            // so this would be unnecessary
            if column_line_limit != 1{
                // revert to last aspect ratio
                    column_line_limit -= 1;
            }

            // determine required number of columns
                required_columns = line_count / column_line_limit;
                if line_count % column_line_limit != 0{
                    required_columns = required_columns + 1;
                }

            println!("Aspect ratio is {} off from target", (last_checked_aspect_ratio - target_aspect_ratio).abs());
        }

        // remake immutable
            let required_columns = required_columns;
            let column_line_limit = column_line_limit;

    // initialize image
        // determine x
            let imgx: u32 = required_columns * column_width;

        // determine y
            let imgy: u32 = if line_count < column_line_limit{
                line_count * 2
            }else{
                column_line_limit * 2
            };
        
        // Create a new ImgBuf with width: imgx and height: imgy
        let mut imgbuf = RgbImage::new(imgx, imgy);
    
    // initialize vars
        let mut cur_line_x = 0;
        let mut line = String::new();
        let mut line_num: u32 = 0;
        let mut background = Rgb([0,0,0]);

    let tq = tqdm_rs::Tqdm::manual(paths.len());
    let mut path_num = 1;
    for path in &paths{
        println!("{}", path.display());
        tq.update(path_num);
        path_num += 1;
        
        // initialize highlighting themes
            let ss = SyntaxSet::load_defaults_newlines();
            let ts = ThemeSet::load_defaults();
            let mut highlighter = HighlightFile::new(path, &ss, &ts.themes["Solarized (dark)"]).unwrap();

        while highlighter.reader.read_line(&mut line).unwrap() > 0 {
            {
                // get position of current line
                    let cur_y = (line_num % column_line_limit) * 2;
                    let cur_column_x_offset = (line_num / column_line_limit) * column_width;

                let regions: Vec<(Style, &str)> = highlighter.highlight_lines.highlight(&line, &ss);

                background = Rgb([regions[0].0.background.r, regions[0].0.background.g, regions[0].0.background.b]);

                for region in regions{

                    let char_color: Rgb<u8> = Rgb([region.0.foreground.r, region.0.foreground.g, region.0.foreground.b]);
                    
                    for chr in region.1.chars(){
                        if cur_line_x >= column_width || region.1.chars().count() == 0{
                            break;
                        }
        
                        // place pixel for character
                            if chr == ' ' || chr == '\n' {
                                imgbuf.put_pixel(cur_column_x_offset + cur_line_x, cur_y, background);
                                imgbuf.put_pixel(cur_column_x_offset + cur_line_x, cur_y + 1, background);
                            }else{
                                imgbuf.put_pixel(cur_column_x_offset + cur_line_x, cur_y, char_color);
                                imgbuf.put_pixel(cur_column_x_offset + cur_line_x, cur_y + 1, char_color);
                            }

                        cur_line_x = cur_line_x + 1;
                    }
                }

                while cur_line_x < column_width{
                    imgbuf.put_pixel(cur_column_x_offset + cur_line_x, cur_y, background);
                    imgbuf.put_pixel(cur_column_x_offset + cur_line_x, cur_y + 1, background);

                    cur_line_x = cur_line_x + 1;
                }

                cur_line_x = 0;
                line_num = line_num + 1;

            } // until NLL this scope is needed so we can clear the buffer after
            line.clear(); // read_line appends so we need to clear between lines
        }
    }

    // fill in the bottom right corner
        while line_num < column_line_limit * required_columns {
            // get position of current line
                let cur_y = (line_num % column_line_limit) * 2;
                let cur_column_x_offset = (line_num / column_line_limit) * column_width;

            // fill line with background color
                for cur_line_x in 0..column_width{
                    imgbuf.put_pixel(cur_column_x_offset + cur_line_x, cur_y, background);
                    imgbuf.put_pixel(cur_column_x_offset + cur_line_x, cur_y + 1, background);
                }

            line_num = line_num + 1;
        }

    imgbuf.save("output.png").unwrap();
}
