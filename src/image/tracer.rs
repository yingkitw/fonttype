use image::GrayImage;

/// Trace the outer boundary of a monochrome image and return contours.
/// Dark pixels (below threshold) are considered inside the shape.
pub fn trace_image(img: &GrayImage, threshold: u8) -> Vec<Vec<(i16, i16)>> {
    let (width, height) = (img.width() as i16, img.height() as i16);
    if width == 0 || height == 0 {
        return vec![];
    }

    let is_inside = |x: i16, y: i16| -> bool {
        if x < 0 || x >= width || y < 0 || y >= height {
            false
        } else {
            img.get_pixel(x as u32, y as u32)[0] < threshold
        }
    };

    let mut visited = vec![vec![false; width as usize]; height as usize];
    let mut contours = Vec::new();

    // Simple boundary following: find top-left pixel of each region
    for y in 0..height {
        for x in 0..width {
            if is_inside(x, y) && !visited[y as usize][x as usize] {
                if !is_inside(x, y - 1) || y == 0 {
                    // Start of a new contour
                    if let Some(contour) = follow_boundary(x, y, is_inside, &mut visited) {
                        contours.push(contour);
                    }
                }
            }
        }
    }

    // If no dark pixels found, return empty
    if contours.is_empty() {
        return vec![];
    }

    // Simplify contours by removing collinear points
    contours.into_iter().map(simplify_contour).filter(|c| !c.is_empty()).collect()
}

fn follow_boundary<F>(start_x: i16, start_y: i16, is_inside: F, visited: &mut Vec<Vec<bool>>) -> Option<Vec<(i16, i16)>>
where
    F: Fn(i16, i16) -> bool,
{
    // 4-connected boundary following (right-hand rule)
    let directions = [(1, 0), (0, 1), (-1, 0), (0, -1)]; // right, down, left, up
    let mut contour = Vec::new();
    let mut x = start_x;
    let mut y = start_y;
    let mut dir = 0; // start going right

    let max_steps = 10000;
    for _ in 0..max_steps {
        visited[y as usize][x as usize] = true;
        contour.push((x, y));

        // Try turning right first, then straight, then left, then back
        let mut moved = false;
        for turn in [3, 0, 1, 2] {
            // right turn = -1 mod 4 = 3, straight = 0, left = 1, back = 2
            let new_dir = (dir + turn) % 4;
            let (dx, dy) = directions[new_dir];
            let nx = x + dx;
            let ny = y + dy;
            if is_inside(nx, ny) {
                x = nx;
                y = ny;
                dir = new_dir;
                moved = true;
                break;
            }
        }

        if !moved {
            break;
        }

        if x == start_x && y == start_y {
            break;
        }
    }

    if contour.len() < 3 {
        None
    } else {
        Some(contour)
    }
}

fn simplify_contour(contour: Vec<(i16, i16)>) -> Vec<(i16, i16)> {
    if contour.len() < 3 {
        return contour;
    }
    let mut result = vec![contour[0]];
    for i in 1..contour.len() - 1 {
        let prev = result.last().copied().unwrap();
        let curr = contour[i];
        let next = contour[i + 1];
        // Check if prev -> curr -> next are collinear
        let dx1 = curr.0 - prev.0;
        let dy1 = curr.1 - prev.1;
        let dx2 = next.0 - curr.0;
        let dy2 = next.1 - curr.1;
        // Not collinear if direction changes
        if dx1 * dy2 != dy1 * dx2 {
            result.push(curr);
        }
    }
    result.push(contour[contour.len() - 1]);
    // Close the contour by making first and last identical
    if result.len() > 1 && result[0] != *result.last().unwrap() {
        let first = result[0];
        result.push(first);
    }
    result
}

/// Create a simple rectangular glyph from image dimensions.
pub fn image_to_rectangle_glyph(img: &GrayImage) -> Vec<Vec<(i16, i16)>> {
    let w = img.width() as i16;
    let h = img.height() as i16;
    vec![vec![
        (0, 0),
        (w, 0),
        (w, h),
        (0, h),
        (0, 0),
    ]]
}
