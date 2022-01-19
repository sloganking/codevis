use std::fs::File;
use std::io::{BufRead, BufReader};
use image::{RgbImage, Rgb};

use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style};
use syntect::easy::HighlightFile;

use syntect;

fn main() {

    // let filename = "src/main.rs";
    let filename = "input/digging.lua";

    // read file (for /n counting)
        let filename = filename;
        // Open the file in read-only mode (ignoring errors).
        let file = File::open(filename).unwrap();
        let reader = BufReader::new(file);

        let line_count = reader.lines().count();
        let line_count = line_count as u32;

    // determine image dimensions based on num of lines and contraints

        // this is a constraint
        let column_line_limit: u32 = 207;

        // determine required number of columns
            let mut required_columns = line_count / column_line_limit;
            if line_count % column_line_limit != 0{
                required_columns = required_columns + 1;
            }

        // remake immutable
        let required_columns = required_columns;

    // initialize image
        // determine x
            let column_width = 100;
            let imgx: u32 = required_columns * column_width;

        // determine y
            let imgy: u32 = if line_count < column_line_limit{
                line_count * 2
            }else{
                column_line_limit * 2
            };
        
        // Create a new ImgBuf with width: imgx and height: imgy
        let mut imgbuf = RgbImage::new(imgx, imgy);

    // initialize highlighting themes
        let ss = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let mut highlighter = HighlightFile::new(filename, &ss, &ts.themes["Solarized (dark)"]).unwrap();

    let mut cur_line_x = 0;
    let mut line = String::new();
    let mut line_num: u32 = 0;
    let mut background = Rgb([0,0,0]);
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
    while line_num < column_line_limit * required_columns {
        // get position of current line
        let cur_y = (line_num % column_line_limit) * 2;
        let cur_column_x_offset = (line_num / column_line_limit) * column_width;

        for cur_line_x in 0..column_width{
            imgbuf.put_pixel(cur_column_x_offset + cur_line_x, cur_y, background);
            imgbuf.put_pixel(cur_column_x_offset + cur_line_x, cur_y + 1, background);
        }

        line_num = line_num + 1;
    }

    // Save the image as “fractal.png”, the format is deduced from the path
    imgbuf.save("output.png").unwrap();

    // test syntacitc highlighting
        // use syntect::parsing::SyntaxSet;
        // use syntect::highlighting::{ThemeSet, Style};
        // use syntect::util::as_24_bit_terminal_escaped;
        // use syntect::easy::HighlightFile;
        // use std::io::BufRead;

        // let ss = SyntaxSet::load_defaults_newlines();
        // let ts = ThemeSet::load_defaults();

        // // ??
        //     // println!("{:?}",ts.themes);

        //     // let keys: Vec<String> = ts.themes.into_keys().collect();
        //     // println!("{:?}",keys);

        //     // for key in ts.themes.IntoKeys(){
        //     // }

        // let mut highlighter = HighlightFile::new("src/main.rs", &ss, &ts.themes["Solarized (dark)"]).unwrap();
        // let mut line = String::new();
        // while highlighter.reader.read_line(&mut line).unwrap() > 0 {
        //     {
        //         let regions: Vec<(Style, &str)> = highlighter.highlight_lines.highlight(&line, &ss);
        //         // println!("{}", as_24_bit_terminal_escaped(&regions[..], true));


        //         for region in regions{
        //             println!("{:?}",region.0.foreground);
        //         }
               

        //     } // until NLL this scope is needed so we can clear the buffer after
        //     line.clear(); // read_line appends so we need to clear between lines
        // }
}
