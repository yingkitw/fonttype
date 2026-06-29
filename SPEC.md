# SPEC

## Overview

`fonttype` is a Rust library and CLI for reading and writing TrueType / OpenType font files.

## Goals

- Parse standard TrueType / OpenType tables
- Serialize font data back to binary
- Provide both a library API and a CLI
- Support round-trip read -> modify -> write

## Supported Formats

| Format | Extension | Status |
|--------|-----------|--------|
| TrueType | `.ttf` | Supported |
| OpenType | `.otf` | Supported |
| WOFF | `.woff` | Supported |
| WOFF2 | `.woff2` | Supported (basic, no table transforms) |
| TrueType Collection | `.ttc` | Supported (read) |

## Supported Tables

- `head` — font header
- `hhea` — horizontal header
- `maxp` — maximum profile
- `post` — PostScript data
- `name` — naming table
- `cmap` — character to glyph mapping (formats 0, 4, 6, 10, 12, 13, 14)
- `OS/2` — OS/2 and Windows metrics
- `glyf` — glyph data (simple and composite)
- `loca` — glyph offsets
- `hmtx` — horizontal metrics
- `kern` — kerning (format 0)
- `GPOS` — kerning pairs
- `GSUB` — feature tags and ligature detection
- `HVAR` / `gvar` — variable font tables (header + passthrough)
- `fvar` — variable font design axes
- `STAT` — style attributes for variable fonts
- `CFF ` — PostScript outlines (header + INDEX parsing, passthrough write)

## CLI Commands

```
fonttype info <file>                          Print font metadata summary
fonttype dump <file>                            Dump all parsed tables to stdout
fonttype create <out.ttf>                       Generate a minimal valid TTF
fonttype create-from-image <img> <cp> <out>     Create font from bitmap
fonttype export-to-image <font> <gid> <out>     Export glyph to PNG
fonttype subset <font> <gid>... <out>           Subset to specified glyphs
fonttype validate <file>                        Validate font checksums
fonttype to-woff <font> <out>                   Convert to WOFF
fonttype from-woff <font> <out>                 Convert from WOFF
fonttype merge <base> <append> <out>            Merge two fonts
fonttype tables <file>                          List all tables
fonttype map <file> <codepoint|gid:N>           Query cmap mapping
fonttype stats <file>                           Show font statistics
fonttype fix <font> <out>                       Rewrite with correct checksums
fonttype extract <font> <table> <out>           Extract table to binary
fonttype inject <font> <table> <data> <out>     Inject raw table
fonttype rename <font> <out> [--family] [--subfamily]  Rename family/subfamily
fonttype strip <font> <out>                     Remove hinting tables
fonttype coverage <file>                        Report Unicode block coverage
fonttype compare <font_a> <font_b>              Structural diff between fonts
fonttype to-woff2 <font> <out>                  Convert to WOFF2
fonttype from-woff2 <font> <out>                Convert from WOFF2
fonttype ttc-info <file>                        List fonts in a TrueType Collection
```

## Library API

```rust
use fonttype::Font;

let font = Font::read(&bytes)?;
println!("{}", font.name.family_name()?);
let out = font.write()?;
std::fs::write("out.ttf", out)?;
```

## Non-Goals (v0.1)

- Hinting execution
- Full OpenType layout (GPOS/GSUB) — only basic kerning pairs and feature tags
- Variable font instance generation
- CFF2 outline editing

## Test Strategy

- Unit tests for each table parser/serializer
- Round-trip tests: parse → serialize → re-parse → compare
