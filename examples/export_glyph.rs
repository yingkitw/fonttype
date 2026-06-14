use fonttype::Font;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let input = args.get(1).map(|s| s.as_str()).unwrap_or("examples/sample.ttf");
    let size: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(256);
    let limit: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(10);

    let bytes = std::fs::read(input)?;
    let font = Font::read(&bytes)?;

    let mut exported = 0usize;
    if let Some(ref glyf) = font.glyf {
        for (i, glyph) in glyf.glyphs.iter().enumerate().take(limit) {
            if let fonttype::tables::glyf::Glyph::Simple(sg) = glyph {
                let path = format!("examples/glyph_{}.png", i);
                fonttype::image::rasterizer::export_glyph_to_image(sg, std::path::Path::new(&path), size)?;
                println!("Exported glyph {} to {}", i, path);
                exported += 1;
            }
        }
    }
    println!("Exported {} glyphs at {}px", exported, size);

    Ok(())
}
