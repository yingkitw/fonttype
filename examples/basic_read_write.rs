use fonttype::Font;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let input = args.get(1).map(|s| s.as_str()).unwrap_or("examples/sample.ttf");
    let output = args.get(2).map(|s| s.as_str()).unwrap_or("examples/out.ttf");

    // Read a font from file
    let bytes = std::fs::read(input)?;
    let font = Font::read(&bytes)?;

    println!("Family: {}", font.name.family_name().unwrap_or_else(|| "?".into()));
    println!("Subfamily: {}", font.name.subfamily_name().unwrap_or_else(|| "?".into()));
    println!("Glyphs: {}", font.maxp.num_glyphs);
    println!("Units/EM: {}", font.head.units_per_em);
    println!("Tables: {}", font.tables.len());

    if let Some(ref fvar) = font.fvar {
        println!("Variable axes: {}", fvar.axes.len());
    }
    if let Some(ref stat) = font.stat {
        println!("STAT axes: {}", stat.design_axes.len());
    }

    // Write it back out (round-trip)
    let out = font.write()?;
    std::fs::write(output, out)?;
    println!("Wrote {}", output);

    // Verify round-trip by re-reading
    let bytes2 = std::fs::read(output)?;
    let font2 = Font::read(&bytes2)?;
    assert_eq!(font.maxp.num_glyphs, font2.maxp.num_glyphs);
    println!("Round-trip verified OK");

    Ok(())
}
