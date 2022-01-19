use std::fs::File;
use std::io::{BufRead, BufReader};
use image::{RgbImage, Rgb};

use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style};
use syntect::easy::HighlightFile;

use syntect;

fn main() {

    // let filename = "src/main.rs";
    let filename = "src/main.rs";

    // read file (for /n counting)
        let filename = filename;
        // Open the file in read-only mode (ignoring errors).
        let file = File::open(filename).unwrap();
        let reader = BufReader::new(file);

    // initialize image
        let imgx = 100;
        let imgy = reader.lines().count() as u32 * 2;
        
        // Create a new ImgBuf with width: imgx and height: imgy
        let mut imgbuf = RgbImage::new(imgx, imgy);

    // initialize highlighting themes
        let ss = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let mut highlighter = HighlightFile::new(filename, &ss, &ts.themes["Solarized (dark)"]).unwrap();

    let mut cur_column_x = 0;
    let mut line = String::new();
    let mut cur_y = 0;
    while highlighter.reader.read_line(&mut line).unwrap() > 0 {
        {
            let regions: Vec<(Style, &str)> = highlighter.highlight_lines.highlight(&line, &ss);

            let background: Rgb<u8> = Rgb([regions[0].0.background.r, regions[0].0.background.g, regions[0].0.background.b]);

            for region in regions{

                
                let char_color: Rgb<u8> = Rgb([region.0.foreground.r, region.0.foreground.g, region.0.foreground.b]);
                

                for chr in region.1.chars(){
                    if cur_column_x >= imgx || region.1.chars().count() == 0{
                        break;
                    }
    
                    // place pixel for character
                    if chr == ' ' || chr == '\n' {
                        imgbuf.put_pixel(cur_column_x, cur_y, background);
                        imgbuf.put_pixel(cur_column_x, cur_y + 1, background);
                    }else{
                        imgbuf.put_pixel(cur_column_x, cur_y, char_color);
                        imgbuf.put_pixel(cur_column_x, cur_y + 1, char_color);
                    }

                    cur_column_x = cur_column_x + 1;
                }
            }

            while cur_column_x < imgx{
                imgbuf.put_pixel(cur_column_x, cur_y, background);
                imgbuf.put_pixel(cur_column_x, cur_y + 1, background);

                cur_column_x = cur_column_x + 1;
            }

            cur_column_x = 0;
            cur_y = cur_y + 2;

        } // until NLL this scope is needed so we can clear the buffer after
        line.clear(); // read_line appends so we need to clear between lines
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
