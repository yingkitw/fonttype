use crate::error::FontError;
use crate::parse::Parser;
use crate::write::Writer;

#[derive(Debug, Clone, PartialEq)]
pub struct GposKernPair {
    pub left: u16,
    pub right: u16,
    pub x_advance: i16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Gpos {
    pub kerning: Vec<GposKernPair>,
    raw: Vec<u8>,
}

impl Gpos {
    pub fn tag() -> crate::error::Tag {
        crate::error::Tag::new(b"GPOS")
    }

    pub fn parse(data: &[u8]) -> Result<Self, FontError> {
        let raw = data.to_vec();
        let mut kerning = Vec::new();

        let mut p = Parser::new(data, 0);
        let major_version = p.u16()?;
        let _minor_version = p.u16()?;
        let _script_list_offset = p.u16()? as usize;
        let _feature_list_offset = p.u16()? as usize;
        let lookup_list_offset = p.u16()? as usize;

        if major_version >= 1 && lookup_list_offset > 0 && lookup_list_offset < data.len() {
            let _ = Self::parse_lookup_list(data, lookup_list_offset, &mut kerning);
        }

        Ok(Gpos { kerning, raw })
    }

    fn parse_lookup_list(data: &[u8], offset: usize, kerning: &mut Vec<GposKernPair>) -> Result<(), FontError> {
        let mut p = Parser::new(data, offset);
        let lookup_count = p.u16()? as usize;
        for _ in 0..lookup_count {
            let lookup_offset = p.u16()? as usize;
            let _ = Self::parse_lookup(data, offset + lookup_offset, kerning);
        }
        Ok(())
    }

    fn parse_lookup(data: &[u8], offset: usize, kerning: &mut Vec<GposKernPair>) -> Result<(), FontError> {
        let mut p = Parser::new(data, offset);
        let lookup_type = p.u16()?;
        let _lookup_flag = p.u16()?;
        let subtable_count = p.u16()? as usize;

        if lookup_type == 2 {
            for _ in 0..subtable_count {
                let subtable_offset = p.u16()? as usize;
                let _ = Self::parse_pair_pos_subtable(data, offset + subtable_offset, kerning);
            }
        }
        Ok(())
    }

    fn parse_pair_pos_subtable(data: &[u8], offset: usize, kerning: &mut Vec<GposKernPair>) -> Result<(), FontError> {
        let mut p = Parser::new(data, offset);
        let pos_format = p.u16()?;
        let coverage_offset = p.u16()? as usize;
        let value_format1 = p.u16()?;
        let value_format2 = p.u16()?;

        let coverage = Self::parse_coverage(data, offset + coverage_offset).unwrap_or_default();

        if pos_format == 1 {
            let pair_set_count = p.u16()? as usize;
            for i in 0..pair_set_count {
                let pair_set_offset = p.u16()? as usize;
                let left_glyph = coverage.get(i).copied().unwrap_or(0);
                let _ = Self::parse_pair_set(data, offset + pair_set_offset, left_glyph, value_format1, value_format2, kerning);
            }
        }
        // pos_format == 2 (class-based) skipped for now
        Ok(())
    }

    fn parse_pair_set(data: &[u8], offset: usize, left_glyph: u16, value_format1: u16, value_format2: u16, kerning: &mut Vec<GposKernPair>) -> Result<(), FontError> {
        let mut p = Parser::new(data, offset);
        let pair_value_count = p.u16()? as usize;
        for _ in 0..pair_value_count {
            let right_glyph = p.u16()?;
            let mut x_advance = 0i16;

            if value_format1 & 0x0001 != 0 { let _ = p.i16(); }
            if value_format1 & 0x0002 != 0 { let _ = p.i16(); }
            if value_format1 & 0x0004 != 0 { x_advance = p.i16()?; }
            if value_format1 & 0x0008 != 0 { let _ = p.i16(); }
            if value_format1 & 0x0010 != 0 { let _ = p.u16(); }
            if value_format1 & 0x0020 != 0 { let _ = p.u16(); }
            if value_format1 & 0x0040 != 0 { let _ = p.u16(); }
            if value_format1 & 0x0080 != 0 { let _ = p.u16(); }

            if value_format2 & 0x0001 != 0 { let _ = p.i16(); }
            if value_format2 & 0x0002 != 0 { let _ = p.i16(); }
            if value_format2 & 0x0004 != 0 { let _ = p.i16(); }
            if value_format2 & 0x0008 != 0 { let _ = p.i16(); }
            if value_format2 & 0x0010 != 0 { let _ = p.u16(); }
            if value_format2 & 0x0020 != 0 { let _ = p.u16(); }
            if value_format2 & 0x0040 != 0 { let _ = p.u16(); }
            if value_format2 & 0x0080 != 0 { let _ = p.u16(); }

            kerning.push(GposKernPair { left: left_glyph, right: right_glyph, x_advance });
        }
        Ok(())
    }

    fn parse_coverage(data: &[u8], offset: usize) -> Result<Vec<u16>, FontError> {
        let mut p = Parser::new(data, offset);
        let format = p.u16()?;
        let count = p.u16()? as usize;
        let mut glyphs = Vec::with_capacity(count);
        if format == 1 {
            for _ in 0..count {
                glyphs.push(p.u16()?);
            }
        } else if format == 2 {
            for _ in 0..count {
                let start = p.u16()?;
                let end = p.u16()?;
                let _index = p.u16()?;
                for g in start..=end {
                    glyphs.push(g);
                }
            }
        }
        Ok(glyphs)
    }

    pub fn write(&self, writer: &mut Writer) -> Result<(), FontError> {
        writer.write_bytes(&self.raw);
        Ok(())
    }
}
