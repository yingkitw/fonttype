use fonttype::Font;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: subset_font <input.ttf> <gid1> <gid2> ... <output.ttf>");
        std::process::exit(1);
    }

    let input = &args[1];
    let output = &args[args.len() - 1];
    let gids: Vec<u16> = args[2..args.len() - 1]
        .iter()
        .map(|s| s.parse().expect("Invalid glyph ID"))
        .collect();

    let bytes = std::fs::read(input)?;
    let font = Font::read(&bytes)?;
    println!("Original: {} glyphs", font.maxp.num_glyphs);

    let subset = font.subset(&gids);
    std::fs::write(output, subset.write()?)?;
    println!("Subset:   {} glyphs -> {}", subset.maxp.num_glyphs, output);

    Ok(())
}
