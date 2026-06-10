# ARCHITECTURE

## Directory Layout

```
.
├── Cargo.toml
├── README.md
├── SPEC.md
├── TODO.md
├── ARCHITECTURE.md
└── src
    ├── lib.rs          — public exports
    ├── main.rs         — CLI entry point
    ├── error.rs        — Error types
    ├── parse.rs        — Binary reading helpers (big-endian, offsets)
    ├── write.rs        — Binary writing helpers
    ├── font.rs         — Core Font struct and table directory
    └── tables
        ├── mod.rs      — Table trait and registry
        ├── head.rs     — head table
        ├── hhea.rs     — hhea table
        ├── maxp.rs     — maxp table
        ├── post.rs     — post table
        ├── name.rs     — name table
        ├── cmap.rs     — cmap table
        └── os2.rs      — OS/2 table
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
