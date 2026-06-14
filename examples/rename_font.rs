use fonttype::Font;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: rename_font <input.ttf> <new_family> <new_subfamily> <output.ttf>");
        std::process::exit(1);
    }

    let input = &args[1];
    let new_family = &args[2];
    let new_subfamily = &args[3];
    let output = &args[4];

    let bytes = std::fs::read(input)?;
    let mut font = Font::read(&bytes)?;

    println!("Before: {} {}",
        font.name.family_name().unwrap_or_else(|| "?".into()),
        font.name.subfamily_name().unwrap_or_else(|| "?".into()));

    font.name.set_family(new_family);
    font.name.set_subfamily(new_subfamily);

    std::fs::write(output, font.write()?)?;

    // Verify
    let bytes2 = std::fs::read(output)?;
    let font2 = Font::read(&bytes2)?;
    println!("After:  {} {}",
        font2.name.family_name().unwrap_or_else(|| "?".into()),
        font2.name.subfamily_name().unwrap_or_else(|| "?".into()));

    Ok(())
}
