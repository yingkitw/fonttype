use fonttype::Font;
use fonttype::tables::glyf::Glyph;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let input = args.get(1).map(|s| s.as_str()).unwrap_or("examples/shape.png");
    let output = args.get(2).map(|s| s.as_str()).unwrap_or("examples/from_image.ttf");

    // Load an image and trace its outline
    let img = image::open(input)?.to_luma8();
    let contours = fonttype::image::tracer::trace_image(&img, 128);

    if contours.is_empty() {
        eprintln!("No contours found in image. Ensure it has a dark shape on a light background.");
        std::process::exit(1);
    }

    // Build a font with the traced glyph at codepoint U+0041 ('A')
    let glyph = Glyph::from_points(contours);
    let mut font = Font::create_minimal();

    if let Some(ref mut glyf) = font.glyf {
        glyf.glyphs.push(glyph);
    }
    font.maxp.num_glyphs = 2;
    font.hhea.number_of_hmetrics = 2;

    // Write the font (loca is regenerated automatically during write)
    std::fs::write(output, font.write()?)?;
    println!("Created {}", output);

    Ok(())
}
