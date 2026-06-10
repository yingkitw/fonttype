use fonttype::Font;
use fonttype::tables::glyf::Glyph;
use fonttype::tables::loca::LocaTable;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load an image and trace its outline
    let img = image::open("examples/shape.png")?.to_luma8();
    let contours = fonttype::image::tracer::trace_image(&img, 128);

    // Build a font with the traced glyph at codepoint U+0041 ('A')
    let glyph = Glyph::from_points(contours);
    let mut font = Font::create_minimal();

    if let Some(ref mut glyf) = font.glyf {
        glyf.glyphs.push(glyph);
    }
    if let Some(ref mut loca) = font.loca {
        let sizes = if let Some(ref glyf) = font.glyf {
            glyf.glyphs.iter().map(|g| {
                let mut w = fonttype::write::Writer::new();
                g.write(&mut w);
                w.len()
            }).collect()
        } else {
            vec![]
        };
        *loca = LocaTable::from_glyph_sizes(&sizes, true);
    }
    font.maxp.num_glyphs = 2;
    font.hhea.number_of_hmetrics = 2;

    // Write the font
    std::fs::write("examples/from_image.ttf", font.write()?)?;
    println!("Created examples/from_image.ttf");

    Ok(())
}
