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
| WOFF2 | `.woff2` | Planned |

## Supported Tables (v0.1)

- `head` — font header
- `hhea` — horizontal header
- `maxp` — maximum profile
- `post` — PostScript data
- `name` — naming table
- `cmap` — character to glyph mapping (formats 0, 4, 12)
- `OS/2` — OS/2 and Windows metrics

## CLI Commands

```
fonttype info <file>       Print font metadata summary
fonttype dump <file>         Dump all parsed tables to stdout
fonttype create <out.ttf>    Generate a minimal valid TTF
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

- Glyph outline rasterization
- Hinting execution
- Full OpenType layout (GPOS/GSUB)
- Variable font instance generation
- Font subsetting

## Test Strategy

- Unit tests for each table parser/serializer
- Round-trip tests: parse → serialize → re-parse → compare
