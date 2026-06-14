use fonttype::Font;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let input = args.get(1).map(|s| s.as_str()).unwrap_or("examples/sample.ttf");

    let bytes = std::fs::read(input)?;
    let font = Font::read(&bytes)?;

    println!("File: {}", input);
    println!("  Family:      {}", font.name.family_name().unwrap_or_else(|| "?".into()));
    println!("  Subfamily:   {}", font.name.subfamily_name().unwrap_or_else(|| "?".into()));
    println!("  Full name:   {}", font.name.full_name().unwrap_or_else(|| "?".into()));
    println!("  Version:     {}", font.name.version().unwrap_or_else(|| "?".into()));
    println!("  Glyphs:      {}", font.maxp.num_glyphs);
    println!("  Units/EM:    {}", font.head.units_per_em);
    println!("  Ascender:    {}", font.hhea.ascender);
    println!("  Descender:   {}", font.hhea.descender);
    println!("  Tables:      {}", font.tables.len());

    if let Some(ref gpos) = font.gpos {
        println!("  GPOS pairs:  {}", gpos.kerning.len());
    }
    if let Some(ref gsub) = font.gsub {
        println!("  GSUB features: {}", gsub.features.join(", "));
        println!("  Has ligatures: {}", gsub.has_ligatures());
    }
    if let Some(ref fvar) = font.fvar {
        for axis in &fvar.axes {
            println!("  Axis {}: {:.0} - {:.0} (default {:.0})",
                axis.axis_tag, axis.min_value, axis.max_value, axis.default_value);
        }
    }
    if let Some(ref stat) = font.stat {
        println!("  STAT axes: {}", stat.design_axes.len());
        println!("  STAT values: {}", stat.axis_values.len());
    }

    Ok(())
}
