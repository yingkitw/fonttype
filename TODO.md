# TODO

## In Progress

## Planned

- [ ] Parse `hvar` / `gvar` tables (variable fonts)
- [ ] WOFF / WOFF2 support
- [ ] Merging fonts

## Done

- [x] Initialize Rust project
- [x] Create README, SPEC, ARCHITECTURE
- [x] Design core `Font` model and table directory
- [x] Implement `read` module: parse sfnt header and table records
- [x] Implement `head` table parsing
- [x] Implement `hhea` table parsing
- [x] Implement `maxp` table parsing
- [x] Implement `post` table parsing
- [x] Implement `name` table parsing
- [x] Implement `cmap` table parsing (format 0, 4, 12)
- [x] Implement `OS/2` table parsing
- [x] Implement `glyf` table parsing/writing (simple glyphs)
- [x] Implement `loca` table parsing/writing
- [x] Implement `write` module: serialize sfnt and tables
- [x] Build CLI (`info`, `dump`, `create`)
- [x] Add image-to-glyph tracer (`create-from-image`)
- [x] Add glyph-to-image rasterizer (`export-to-image`)
- [x] Add comprehensive unit tests (round-trip, glyph, image, rasterizer)
- [x] Add examples
- [x] Implement `hmtx` table parsing/writing
- [x] Implement `kern` table parsing/writing (format 0)
- [x] Add font subsetting (`subset`)
- [x] Add font validation (`validate`) with checksum verification
- [x] Parse `cvt `, `prep`, `fpgm` hinting tables (raw byte passthrough)
- [x] Parse `GPOS` table — extract PairPos kerning pairs
- [x] Parse `GSUB` table — extract feature tags and ligature detection
- [x] Parse `HVAR` / `gvar` tables — header + raw passthrough
- [x] WOFF read/write support (`to-woff`, `from-woff`)
- [x] Font merging (`merge` command)
- [x] Round-trip test suite with real font files (Geneva.ttf read + validate)
