use fonttype::Font;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read("examples/sample.ttf")?;
    let font = Font::read(&bytes)?;

    if let Some(ref glyf) = font.glyf {
        for (i, glyph) in glyf.glyphs.iter().enumerate() {
            if let fonttype::tables::glyf::Glyph::Simple(sg) = glyph {
                let path = format!("examples/glyph_{}.png", i);
                fonttype::image::rasterizer::export_glyph_to_image(sg, std::path::Path::new(&path), 256)?;
                println!("Exported glyph {} to {}", i, path);
            }
        }
    }

    Ok(())
}
