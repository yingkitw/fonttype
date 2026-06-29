# fonttype

A Rust library and CLI for reading and writing TrueType / OpenType font files.

## Quick Start

```bash
# Build
cargo build --release

# Inspect a font
./target/release/fonttype info MyFont.ttf

# Dump parsed tables
./target/release/fonttype dump MyFont.ttf

# Create a minimal test font
./target/release/fonttype create out.ttf

# Create a font from a bitmap image
./target/release/fonttype create-from-image shape.png 65 out.ttf

# Export a glyph to a PNG
./target/release/fonttype export-to-image out.ttf 1 glyph.png --size 256

# List tables
./target/release/fonttype tables MyFont.ttf

# Query codepoint mapping
./target/release/fonttype map MyFont.ttf A
./target/release/fonttype map MyFont.ttf gid:36

# Show font statistics
./target/release/fonttype stats MyFont.ttf

# Fix checksums and rewrite
./target/release/fonttype fix MyFont.ttf fixed.ttf

# Extract a single table
./target/release/fonttype extract MyFont.ttf head head.bin

# Inject a raw table
./target/release/fonttype inject MyFont.ttf MATH math.bin out.ttf

# Rename family/subfamily
./target/release/fonttype rename MyFont.ttf --family "NewName" out.ttf

# Strip hinting tables and glyph instructions
./target/release/fonttype strip MyFont.ttf out.ttf

# Report Unicode block coverage
./target/release/fonttype coverage MyFont.ttf

# Compare two fonts structurally
./target/release/fonttype compare A.ttf B.ttf

# Convert to/from WOFF2
./target/release/fonttype to-woff2 MyFont.ttf MyFont.woff2
./target/release/fonttype from-woff2 MyFont.woff2 MyFont.ttf

# List fonts in a TrueType Collection
./target/release/fonttype ttc-info MyCollection.ttc

# Dump font as TTX-style XML
./target/release/fonttype ttx MyFont.ttf
```

## Library Usage

### Read / Write

```rust
use fonttype::Font;

let font = Font::read(&std::fs::read("MyFont.ttf")?)?;
println!("Family: {}", font.name.family_name().unwrap_or_else(|| "?".into()));

std::fs::write("out.ttf", font.write()?)?;
```

### Create from Image

```rust
use fonttype::Font;
use fonttype::tables::glyf::Glyph;

let img = image::open("shape.png")?.to_luma8();
let contours = fonttype::image::tracer::trace_image(&img, 128);
let glyph = Glyph::from_points(contours);

let mut font = Font::create_minimal();
font.glyf.as_mut().unwrap().glyphs.push(glyph);
std::fs::write("out.ttf", font.write()?)?;
```

### Export Glyph to Image

```rust
use fonttype::Font;

let font = Font::read(&std::fs::read("MyFont.ttf")?)?;
if let Some(ref glyf) = font.glyf {
    if let fonttype::tables::glyf::Glyph::Simple(ref sg) = glyf.glyphs[0] {
        fonttype::image::rasterizer::export_glyph_to_image(
            sg, std::path::Path::new("glyph.png"), 256
        )?;
    }
}
```

## Supported Tables

| Table | Read | Write |
|-------|------|-------|
| `head` | Yes | Yes |
| `hhea` | Yes | Yes |
| `maxp` | Yes | Yes |
| `post` | Yes | Yes |
| `name` | Yes | Yes |
| `cmap` | Yes | Yes |
| `OS/2` | Yes | Yes |
| `glyf` | Yes (simple + composite) | Yes (simple + composite) |
| `loca` | Yes | Yes |
| `hmtx` | Yes | Yes |
| `kern` | Yes (format 0) | Yes (format 0) |
| `GPOS` | Yes (kerning pairs) | Yes |
| `GSUB` | Yes (features, ligatures) | Yes |
| `HVAR` | Yes (header) | Yes (passthrough) |
| `gvar` | Yes (header) | Yes (passthrough) |
| `fvar` | Yes | Yes |
| `STAT` | Yes | Yes |
| `CFF ` | Yes (header + INDEX) | Yes (passthrough) |
| `COLR` | Yes (v0) | Yes |
| `CPAL` | Yes (v0) | Yes |
| `SVG ` | Yes (v0) | Yes |

## Formats

| Format | Read | Write |
|--------|------|-------|
| TrueType / OpenType (sfnt) | Yes | Yes |
| WOFF | Yes | Yes |
| WOFF2 | Yes | Yes (basic, no table transforms) |
| TrueType Collection (.ttc) | Yes | — |

## Examples

See `examples/` directory:
- `basic_read_write.rs` — read a font, inspect metadata, write back
- `create_from_image.rs` — load a bitmap, trace outline, embed as glyph
- `export_glyph.rs` — render all glyphs to PNG files
- `inspect_font.rs` — comprehensive font metadata inspection
- `subset_font.rs` — programmatic glyph subsetting
- `rename_font.rs` — modify family and subfamily names
- `extract_table.rs` — extract a raw table by tag

## Architecture

See `ARCHITECTURE.md` and `SPEC.md`.

## License

MIT
