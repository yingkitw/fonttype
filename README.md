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
| `glyf` | Yes (simple) | Yes (simple) |
| `loca` | Yes | Yes |

## Examples

See `examples/` directory:
- `basic_read_write.rs` — read a font, inspect metadata, write back
- `create_from_image.rs` — load a bitmap, trace outline, embed as glyph
- `export_glyph.rs` — render all glyphs to PNG files

## Architecture

See `ARCHITECTURE.md` and `SPEC.md`.

## License

MIT
