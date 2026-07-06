# ARCHITECTURE

## Directory Layout

```
.
├── Cargo.toml
├── README.md
├── SPEC.md
├── TODO.md
├── ARCHITECTURE.md
├── examples/          — standalone usage examples
├── tests/
│   └── integration.rs — end-to-end tests (round-trip, parsing, format coverage)
└── src
    ├── lib.rs          — public exports
    ├── main.rs         — CLI entry point
    ├── error.rs        — Error types
    ├── parse.rs        — Binary reading helpers (big-endian, offsets)
    ├── write.rs        — Binary writing helpers
    ├── font.rs         — Core Font struct and table directory
    ├── woff.rs         — WOFF read/write
    ├── woff2.rs        — WOFF2 read/write (Brotli)
    ├── ttc.rs          — TrueType Collection parsing
    ├── bezier.rs       — Bezier curve editor (kurbo-backed)
    ├── encoding.rs     — Legacy char encodings (Latin-1, Windows-1252, Mac Roman)
    ├── modifier.rs     — Builder-style FontModifier API
    ├── validation.rs   — Structured ValidationReport types
    ├── image
    │   ├── mod.rs      — module root
    │   ├── tracer.rs   — bitmap → glyph outline tracer
    │   └── rasterizer.rs — glyph → PNG rasterizer
    └── tables
        ├── mod.rs           — Table trait and module registry
        ├── head.rs          — head table
        ├── hhea.rs          — hhea table
        ├── maxp.rs          — maxp table
        ├── post.rs          — post table
        ├── name.rs          — name table
        ├── cmap.rs          — cmap table (formats 0, 4, 6, 10, 12, 13, 14)
        ├── os2.rs           — OS/2 table
        ├── glyf.rs          — glyph data (simple + composite)
        ├── loca.rs          — glyph offsets
        ├── hmtx.rs          — horizontal metrics
        ├── kern.rs          — kerning
        ├── hinting.rs       — cvt / prep / fpgm (raw byte passthrough)
        ├── gpos.rs          — GPOS (kerning pairs)
        ├── gsub.rs          — GSUB (ScriptList, FeatureList, LookupList)
        ├── var.rs           — HVAR / gvar passthrough
        ├── fvar.rs          — variable font axes
        ├── stat.rs          — STAT style attributes
        ├── cff.rs           — CFF / CFF2 PostScript outlines (header + INDEX)
        ├── cff_charstring.rs — CFF Type 2 CharString decoder
        ├── colr.rs          — COLR color glyph layers (v0)
        ├── cpal.rs          — CPAL color palettes (v0)
        └── svg.rs           — SVG glyph artwork (v0)
```

## Core Abstractions

### `Font`

The root struct. Holds a `Vec<TableRecord>` (directory) and parsed table structs.

```rust
pub struct Font {
    pub sfnt_version: SfntVersion,
    pub tables: Vec<TableRecord>,
    pub head: Head,
    pub hhea: Hhea,
    pub maxp: Maxp,
    pub post: Post,
    pub name: Name,
    pub cmap: Cmap,
    pub os2: Os2,
    pub glyf: Option<GlyfTable>,
    pub loca: Option<LocaTable>,
    pub hmtx: Hmtx,
    pub kern: Option<Kern>,
    pub cvt: Option<Vec<u8>>,
    pub prep: Option<Vec<u8>>,
    pub fpgm: Option<Vec<u8>>,
    pub gpos: Option<Gpos>,
    pub gsub: Option<Gsub>,
    pub hvar: Option<Hvar>,
    pub gvar: Option<Gvar>,
    pub fvar: Option<Fvar>,
    pub stat: Option<Stat>,
    pub cff: Option<Cff>,
    pub colr: Option<Colr>,
    pub cpal: Option<Cpal>,
    pub svg: Option<Svg>,
    pub raw_tables: Vec<(Tag, Vec<u8>)>,
}
```

### `TableRecord`

Raw directory entry: tag, checksum, offset, length.

### `Table` trait

```rust
pub trait Table: Sized {
    fn tag() -> Tag;
    fn parse(buf: &[u8], offset: usize) -> Result<Self>;
    fn write(&self, writer: &mut Vec<u8>) -> Result<()>;
}
```

## Parsing Strategy

1. Read the first 12 bytes: sfnt version + numTables + searchRange + entrySelector + rangeShift
2. Read `numTables` × 16-byte `TableRecord`s
3. For each known table, seek to its offset and parse with the corresponding `Table::parse`
4. Unknown tables are preserved as raw bytes for round-trip fidelity

## Writing Strategy

1. Compute table offsets and pad to 4-byte boundaries
2. Write the font header
3. Write the table directory
4. Write each table body, recalculating checksums
5. Update `head.checkSumAdjustment` last

## Error Handling

All fallible operations return `FontError`, a `thiserror` enum covering:

- `Io(std::io::Error)`
- `InvalidTable { tag: Tag, reason: String }`
- `MissingTable(Tag)`
- `UnsupportedCmapFormat(u16)`
- `ChecksumMismatch`

## Dependencies

- `byteorder` — big-endian integer parsing
- `thiserror` — ergonomic error types
- `clap` — CLI argument parsing
- `flate2` — WOFF zlib compression/decompression
- `brotli` — WOFF2 Brotli compression/decompression
- `image` — PNG rasterization and bitmap tracing
- `kurbo` — 2D bezier path geometry used by `bezier.rs`
