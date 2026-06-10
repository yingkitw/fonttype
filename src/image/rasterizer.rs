use image::{GrayImage, Luma};
use crate::tables::glyf::SimpleGlyph;

/// Rasterize a simple glyph to a grayscale image.
/// Fills the polygonal contours with white (255) on a black (0) background.
pub fn rasterize_glyph(glyph: &SimpleGlyph, width: u32, height: u32) -> GrayImage {
    let mut img = GrayImage::from_pixel(width, height, Luma([0]));
    let scale_x = width as f32 / (glyph.x_max - glyph.x_min).max(1) as f32;
    let scale_y = height as f32 / (glyph.y_max - glyph.y_min).max(1) as f32;
    let scale = scale_x.min(scale_y);
    let offset_x = -(glyph.x_min as f32 * scale);
    let offset_y = -(glyph.y_min as f32 * scale);

    // Reconstruct absolute coordinates from relative deltas
    let mut points: Vec<(f32, f32)> = Vec::new();
    let mut x = 0i16;
    let mut y = 0i16;
    let mut contour_idx = 0;
    for (i, &flag) in glyph.flags.iter().enumerate() {
        let dx = glyph.x_coordinates[i];
        let dy = glyph.y_coordinates[i];
        if flag & 0x02 != 0 {
            // short vector
            let sx = if flag & 0x10 != 0 { dx } else { -dx };
            let sy = if flag & 0x20 != 0 { dy } else { -dy };
            x += sx;
            y += sy;
        } else {
            if flag & 0x10 != 0 {
                // same as previous (0 delta)
            } else {
                x += dx;
                y += dy;
            }
        }
        points.push((x as f32 * scale + offset_x, y as f32 * scale + offset_y));
        if contour_idx < glyph.end_pts_of_contours.len() && i == glyph.end_pts_of_contours[contour_idx] as usize {
            // End of contour: fill it
            let contour_points = &points[..=glyph.end_pts_of_contours[contour_idx] as usize];
            fill_polygon(contour_points, &mut img);
            contour_idx += 1;
        }
    }
    img
}

fn fill_polygon(points: &[(f32, f32)], img: &mut GrayImage) {
    if points.len() < 3 {
        return;
    }
    let height = img.height() as i32;
    let width = img.width() as i32;

    for y in 0..height {
        let mut intersections = Vec::new();
        let scan_y = y as f32;
        for i in 0..points.len() {
            let j = (i + 1) % points.len();
            let (x1, y1) = points[i];
            let (x2, y2) = points[j];
            // Check if edge crosses scanline
            if (y1 <= scan_y && y2 > scan_y) || (y2 <= scan_y && y1 > scan_y) {
                let t = (scan_y - y1) / (y2 - y1);
                let x = x1 + t * (x2 - x1);
                intersections.push(x);
            }
        }
        intersections.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for pair in intersections.chunks(2) {
            if pair.len() == 2 {
                let x_start = pair[0].max(0.0) as i32;
                let x_end = pair[1].min(width as f32) as i32;
                for x in x_start..x_end {
                    if x >= 0 && x < width {
                        img.put_pixel(x as u32, y as u32, Luma([255]));
                    }
                }
            }
        }
    }
}

/// Export a glyph to a PNG file.
pub fn export_glyph_to_image(glyph: &SimpleGlyph, path: &std::path::Path, size: u32) -> Result<(), image::ImageError> {
    let img = rasterize_glyph(glyph, size, size);
    img.save(path)
}
