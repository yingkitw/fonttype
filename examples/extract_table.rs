use fonttype::Font;
use fonttype::Tag;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: extract_table <input.ttf> <TABLE> <output.bin>");
        std::process::exit(1);
    }

    let input = &args[1];
    let table_tag = args[2].as_bytes();
    let output = &args[3];

    if table_tag.len() != 4 {
        eprintln!("Table tag must be exactly 4 ASCII characters");
        std::process::exit(1);
    }

    let bytes = std::fs::read(input)?;
    let font = Font::read(&bytes)?;
    let tag = Tag::new(table_tag.try_into().unwrap());

    if let Some(rec) = font.tables.iter().find(|t| t.tag == tag) {
        let start = rec.offset as usize;
        let end = start + rec.length as usize;
        std::fs::write(output, &bytes[start..end])?;
        println!("Extracted {} ({} bytes) -> {}", args[2], rec.length, output);
    } else {
        eprintln!("Table {} not found in {}", args[2], input);
        std::process::exit(1);
    }

    Ok(())
}
