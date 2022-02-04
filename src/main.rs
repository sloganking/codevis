use code_visualizer::renderer;

fn main() {
    let paths = renderer::get_unicode_files_in_dir("./input/");

    let line_count = renderer::count_lines_in_files(&paths);

    // render files to image, and store in ./output.png
    renderer::render(&paths, 100, 16.0 / 9.0, true, true, Some(0)).expect("No UTF-8 compatible files found.").save("./output.png").unwrap();

    let tq = tqdm_rs::Tqdm::manual(line_count as usize);
    let mut tq_count = 1;

    for x in 0..line_count{

        // print progress
            tq.update(tq_count);
            tq_count += 1;

        let file_num = format!("{}",x);
        let file_num = format!("{:0>8}",file_num);
        let file_num = format!("./images/{}",file_num);
        // render_to_file(&paths, 100, 16.0 / 10.0, true, , x);

        renderer::render(&paths, 100, 16.0 / 9.0, true, false, Some(x)).expect("No UTF-8 compatible files found.").save(&(file_num + ".png")).unwrap();

    }

    
}