use std::fs::File;
use std::io::{BufRead, BufReader};
use image::{RgbImage, Rgb};

use syntect;

fn main() {

    let background: Rgb<u8> = Rgb([0,0,0]);
    let char_color = image::Rgb([255,255,255]);

    let filename = "input/lib.rs";

    // read file (for /n counting)
        let filename = filename;
        // Open the file in read-only mode (ignoring errors).
        let file = File::open(filename).unwrap();
        let reader = BufReader::new(file);

    // initialize image
        let imgx = 80;
        let imgy = reader.lines().count() as u32 * 2;
        
        // Create a new ImgBuf with width: imgx and height: imgy
        let mut imgbuf = RgbImage::new(imgx, imgy);

    // read file
        let filename = filename;
        // Open the file in read-only mode (ignoring errors).
        let file = File::open(filename).unwrap();
        let reader = BufReader::new(file);

    // fill image with color
        // Iterate over the coordinates and pixels of the image
        for pixel in imgbuf.pixels_mut() {
            *pixel = background;
        }

    // Read the file line by line using the lines() iterator from std::io::BufRead.
    for (index, line) in reader.lines().enumerate() {
        let index = index as u32;

        let line = line.unwrap(); // Ignore errors.


        // Show the line and its number.
        // println!("{}. {}", index + 1, line);

        let cur_y = index * 2;

        // for chars in line
        for (i, chr) in line.chars().enumerate(){
            let i = i as u32;

            if i >= imgx || line.chars().count() == 0{
                break;
            }

            if chr == ' ' {
                imgbuf.put_pixel(i, cur_y, background);
                imgbuf.put_pixel(i, cur_y + 1, background);
            }else{
                imgbuf.put_pixel(i, cur_y, char_color);
                imgbuf.put_pixel(i, cur_y + 1, char_color);
            }
        }
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
        //         println!("{}", as_24_bit_terminal_escaped(&regions[..], true));


        //         // for region in regions{
        //         //     println!("{:?}",region.0.foreground);
        //         // }
               

        //     } // until NLL this scope is needed so we can clear the buffer after
        //     line.clear(); // read_line appends so we need to clear between lines
        // }
}
