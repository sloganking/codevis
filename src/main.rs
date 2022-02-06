use code_visualizer::renderer;
use std::fs;

fn main() {
    let paths = renderer::get_unicode_files_in_dir("./input/");

    // render files to image, and store in ./output.png
    let img = renderer::render(&paths, 100, 16.0 / 9.0, true, true).expect("No UTF-8 compatible files found.");
    
    // img.save("./output.png").unwrap();

    let width = img.width();
    let height = img.height();
    let img_vec = img.to_vec();

    // qoi compression
        use qoi::{encode_to_vec, decode_to_vec};

        let encoded = encode_to_vec(&img_vec, width, height).unwrap();
        let (header, decoded) = decode_to_vec(&encoded).unwrap();

        // qoi assertions
            assert_eq!(header.width, width);
            assert_eq!(header.height, height);
            assert_eq!(decoded, img_vec);
            println!("tests passed!");

    // print stats
        println!("src img size: {}", img_vec.len());
        println!("compresed img size: {}", encoded.len());
        println!("compressed image percentage: {}", encoded.len() as f64 / img_vec.len() as f64);

    // write qoi file
        fs::write("output.qoi", encoded).unwrap();
}