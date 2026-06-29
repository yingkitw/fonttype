# TODO

## In Progress

## Planned

### Phase 1 — CLI Utilities
- [x] `tables` command — list all tables with offset, length, checksum
- [x] `map` command — query cmap: codepoint → glyph ID or glyph ID → codepoint(s)
- [x] `stats` command — show font statistics (glyph count, table sizes, metrics summary)
- [x] `fix` command — rewrite font with recomputed checksums and alignment

### Phase 2 — Parsing Improvements
- [x] Composite glyph parsing in `glyf` (currently stubbed as Empty)
- [x] `fvar` table parsing — variable font design axes
- [x] `STAT` table parsing — style attributes for variable fonts

### Phase 3 — Advanced Manipulation
- [x] `extract` command — dump a single table to a binary file
- [x] `inject` command — replace or insert a table from a binary file
- [x] `rename` command — modify family / subfamily names in the `name` table
- [x] `strip` command — remove hinting tables (`cvt `, `prep`, `fpgm`) to reduce file size
- [x] `coverage` command — report Unicode block coverage
- [x] `compare` command — structural diff between two fonts

### Phase 4 — Format & Infrastructure
- [x] WOFF2 read/write support (Brotli-compressed)
- [x] TrueType Collection (`.ttc`) parsing
- [x] CFF / CFF2 table parsing for PostScript-outline OTFs

## Brainstorming

- TTX-style XML dump / import for human-readable table inspection
- Full GPOS / GSUB parsing (LookupList, ScriptList, FeatureList decomposition)
- Full CFF / CFF2 CharString decoding and outline extraction
- SVG table read/write for color fonts
- COLR / CPAL table support for color glyph layers
- Variable font instance generation (apply fvar + avar + gvar)
- OpenType feature language (`.fea`) parser and compiler
- OpenTypeSanitizer-style structural validation
- Font hinting compiler / auto-hinter
- AAT tables (morx, mort, etc.)
- Better WOFF2 table transform support (glyf / loca transformed)

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
- [x] WOFF2 read/write support (`to-woff2`, `from-woff2`)
- [x] TrueType Collection (`.ttc`) parsing (`ttc-info`)
- [x] CFF / CFF2 table basic parsing for PostScript-outline OTFs
- [x] cmap format 6, 10, 13, 14 parsing and writing
