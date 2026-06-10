use fonttype::Font;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read a font from file
    let bytes = std::fs::read("examples/sample.ttf")?;
    let font = Font::read(&bytes)?;

    println!("Family: {}", font.name.family_name().unwrap_or_else(|| "?".into()));
    println!("Glyphs: {}", font.maxp.num_glyphs);
    println!("Units/EM: {}", font.head.units_per_em);

    // Write it back out
    let out = font.write()?;
    std::fs::write("examples/out.ttf", out)?;
    println!("Wrote examples/out.ttf");

    Ok(())
}
