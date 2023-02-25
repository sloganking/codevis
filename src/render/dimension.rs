pub struct Dimension {
    pub imgx: u32,
    pub imgy: u32,
    pub lines_per_column: u32,
    pub required_columns: u32,
}

/// determine number and height of columns closest to desired aspect ratio
pub(crate) fn compute(
    target_aspect_ratio: f64,
    column_width: u32,
    total_line_count: u32,
    line_height: u32,
    force_full_columns: bool,
    mut progress: impl prodash::Progress,
) -> anyhow::Result<Dimension> {
    // determine image dimensions based on num of lines and constraints
    let mut lines_per_column = 1;
    let mut last_checked_aspect_ratio: f64 = f64::MAX;
    let mut last_column_line_limit = lines_per_column;
    let mut required_columns;

    // determine maximum aspect ratios
    // the width of one column, divided by the combined height of all the lines.
    let tallest_aspect_ratio = column_width as f64 / (total_line_count as f64 * line_height as f64);
    // the combined width of all the columns, divided by the height of one line.
    let widest_aspect_ratio = (column_width as f64 * total_line_count as f64) / line_height as f64;

    // start at widest possible aspect ratio.
    // This will later be made taller until the closest aspect ratio to the target is found.
    let mut cur_aspect_ratio = widest_aspect_ratio;

    if target_aspect_ratio <= tallest_aspect_ratio {
        // use tallest possible aspect ratio
        lines_per_column = total_line_count;
        required_columns = 1;
    } else if target_aspect_ratio >= widest_aspect_ratio {
        // use widest possible aspect ratio
        lines_per_column = 1;
        required_columns = total_line_count;
    } else {
        // start at widest possible aspect ratio
        lines_per_column = 1;
        // required_columns = line_count;

        // de-widen aspect ratio until closest match is found
        while (last_checked_aspect_ratio - target_aspect_ratio).abs()
            > (cur_aspect_ratio - target_aspect_ratio).abs()
        {
            // remember current aspect ratio
            last_checked_aspect_ratio = cur_aspect_ratio;

            if force_full_columns {
                last_column_line_limit = lines_per_column;

                // determine required number of columns
                required_columns = total_line_count / lines_per_column;
                if total_line_count % lines_per_column != 0 {
                    required_columns += 1;
                }

                let last_required_columns = required_columns;

                // find next full column aspect ratio
                while required_columns == last_required_columns {
                    lines_per_column += 1;

                    // determine required number of columns
                    required_columns = total_line_count / lines_per_column;
                    if total_line_count % lines_per_column != 0 {
                        required_columns += 1;
                    }
                }
            } else {
                // generate new aspect ratio

                lines_per_column += 1;

                // determine required number of columns
                required_columns = total_line_count / lines_per_column;
                if total_line_count % lines_per_column != 0 {
                    required_columns += 1;
                }
            }

            cur_aspect_ratio = required_columns as f64 * column_width as f64
                / (lines_per_column as f64 * line_height as f64);
        }

        //> re-determine best aspect ratio

        // (Should never not happen, but)
        // previous while loop would never have been entered if (column_line_limit == 1)
        // so (column_line_limit -= 1;) would be unnecessary
        if lines_per_column != 1 && !force_full_columns {
            // revert to last aspect ratio
            lines_per_column -= 1;
        } else if force_full_columns {
            lines_per_column = last_column_line_limit;
        }

        // determine required number of columns
        required_columns = total_line_count / lines_per_column;
        if total_line_count % lines_per_column != 0 {
            required_columns += 1;
        }
    }

    let imgx: u32 = required_columns * column_width;
    let imgy: u32 = total_line_count.min(lines_per_column) * line_height;

    progress.info(format!(
        "Aspect ratio is {} off from target",
        (last_checked_aspect_ratio - target_aspect_ratio).abs(),
    ));

    Ok(Dimension {
        imgx,
        imgy,
        lines_per_column,
        required_columns,
    })
}
