use crate::error::{FontError, Tag};
use crate::tables::Table;
use crate::write::Writer;

#[derive(Debug, Clone, PartialEq)]
pub struct LocaTable {
    pub offsets: Vec<u32>,
    pub format: u16, // 0 = short offsets / 2, 1 = long offsets
}

impl LocaTable {
    pub fn from_glyph_sizes(sizes: &[usize], _long_format: bool) -> Self {
        let mut offsets = Vec::with_capacity(sizes.len() + 1);
        let mut current = 0u32;
        offsets.push(current);
        for &size in sizes {
            current += size as u32;
            offsets.push(current);
        }
        LocaTable {
            offsets,
            format: 1, // always use long format to avoid odd-byte truncation
        }
    }
}

impl Table for LocaTable {
    fn tag() -> Tag {
        Tag::new(b"loca")
    }

    fn parse(_buf: &[u8], _offset: usize) -> Result<Self, FontError> {
        // Cannot parse loca without knowing numGlyphs and indexToLocFormat
        // Use parse_with_info instead
        Err(FontError::invalid_table(
            Self::tag(),
            "loca requires numGlyphs and indexToLocFormat; use parse_with_info",
        ))
    }

    fn write(&self, w: &mut Writer) -> Result<(), FontError> {
        if self.format == 0 {
            for &off in &self.offsets {
                w.write_u16((off / 2) as u16);
            }
        } else {
            for &off in &self.offsets {
                w.write_u32(off);
            }
        }
        Ok(())
    }
}
