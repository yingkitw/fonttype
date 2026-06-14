use crate::error::{FontError, Tag};
use crate::parse::Parser;
use crate::tables::Table;
use crate::write::Writer;

#[derive(Debug, Clone, PartialEq)]
pub struct Cmap {
    pub version: u16,
    pub num_tables: u16,
    pub records: Vec<EncodingRecord>,
    pub subtables: Vec<CmapSubtable>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EncodingRecord {
    pub platform_id: u16,
    pub encoding_id: u16,
    pub subtable_offset: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CmapSubtable {
    Format0 {
        language: u16,
        glyph_id_array: [u8; 256],
    },
    Format4 {
        language: u16,
        segments: Vec<Format4Segment>,
    },
    Format12 {
        language: u32,
        groups: Vec<SequentialMapGroup>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Format4Segment {
    pub end_code: u16,
    pub start_code: u16,
    pub id_delta: i16,
    pub id_range_offset: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequentialMapGroup {
    pub start_char_code: u32,
    pub end_char_code: u32,
    pub start_glyph_id: u32,
}

impl Cmap {
    pub fn map_codepoint(&self, codepoint: u32) -> Option<u16> {
        for subtable in &self.subtables {
            if let Some(gid) = subtable.map(codepoint) {
                return Some(gid);
            }
        }
        None
    }

    pub fn glyph_codepoints(&self, glyph_id: u16) -> Vec<u32> {
        let mut cps = Vec::new();
        for subtable in &self.subtables {
            cps.extend(subtable.codepoints_for_glyph(glyph_id));
        }
        cps.sort_unstable();
        cps.dedup();
        cps
    }
}

impl CmapSubtable {
    pub fn map(&self, codepoint: u32) -> Option<u16> {
        match self {
            CmapSubtable::Format0 { glyph_id_array, .. } => {
                if codepoint < 256 {
                    Some(glyph_id_array[codepoint as usize] as u16)
                } else {
                    None
                }
            }
            CmapSubtable::Format4 { segments, .. } => {
                if codepoint > 0xFFFF {
                    return None;
                }
                let cp = codepoint as u16;
                for seg in segments {
                    if cp >= seg.start_code && cp <= seg.end_code {
                        if seg.id_range_offset == 0 {
                            return Some((cp as i32 + seg.id_delta as i32) as u16);
                        }
                        // idRangeOffset path omitted for brevity
                    }
                }
                None
            }
            CmapSubtable::Format12 { groups, .. } => {
                for g in groups {
                    if codepoint >= g.start_char_code && codepoint <= g.end_char_code {
                        return Some((g.start_glyph_id + (codepoint - g.start_char_code)) as u16);
                    }
                }
                None
            }
        }
    }

    pub fn codepoints_for_glyph(&self, glyph_id: u16) -> Vec<u32> {
        match self {
            CmapSubtable::Format0 { glyph_id_array, .. } => {
                glyph_id_array.iter().enumerate()
                    .filter(|(_, g)| **g == glyph_id as u8)
                    .map(|(cp, _)| cp as u32)
                    .collect()
            }
            CmapSubtable::Format4 { segments, .. } => {
                let mut cps = Vec::new();
                for seg in segments {
                    if seg.id_range_offset == 0 {
                        let start = seg.start_code as i32;
                        let end = seg.end_code as i32;
                        let delta = seg.id_delta as i32;
                        for cp in start..=end {
                            if ((cp + delta) as u16) == glyph_id {
                                cps.push(cp as u32);
                            }
                        }
                    }
                }
                cps
            }
            CmapSubtable::Format12 { groups, .. } => {
                let mut cps = Vec::new();
                for g in groups {
                    let start_gid = g.start_glyph_id;
                    let end_gid = g.start_glyph_id + (g.end_char_code - g.start_char_code);
                    if glyph_id as u32 >= start_gid && glyph_id as u32 <= end_gid {
                        let offset = glyph_id as u32 - start_gid;
                        cps.push(g.start_char_code + offset);
                    }
                }
                cps
            }
        }
    }
}

impl Table for Cmap {
    fn tag() -> Tag {
        Tag::new(b"cmap")
    }

    fn parse(buf: &[u8], offset: usize) -> Result<Self, FontError> {
        let mut p = Parser::new(buf, offset);
        let version = p.u16()?;
        let num_tables = p.u16()?;
        let mut records = Vec::with_capacity(num_tables as usize);
        let mut subtables = Vec::with_capacity(num_tables as usize);
        for _ in 0..num_tables {
            records.push(EncodingRecord {
                platform_id: p.u16()?,
                encoding_id: p.u16()?,
                subtable_offset: p.u32()?,
            });
        }
        for rec in &records {
            let mut sp = Parser::new(buf, offset + rec.subtable_offset as usize);
            let format = sp.u16()?;
            match format {
                0 => {
                    let _length = sp.u16()?;
                    let language = sp.u16()?;
                    let mut glyph_id_array = [0u8; 256];
                    for i in 0..256 {
                        glyph_id_array[i] = sp.u8()?;
                    }
                    subtables.push(CmapSubtable::Format0 {
                        language,
                        glyph_id_array,
                    });
                }
                4 => {
                    let _length = sp.u16()?;
                    let language = sp.u16()?;
                    let seg_count_x2 = sp.u16()?;
                    let seg_count = (seg_count_x2 / 2) as usize;
                    let _search_range = sp.u16()?;
                    let _entry_selector = sp.u16()?;
                    let _range_shift = sp.u16()?;
                    let mut end_codes = Vec::with_capacity(seg_count);
                    for _ in 0..seg_count {
                        end_codes.push(sp.u16()?);
                    }
                    let _reserved = sp.u16()?;
                    let mut start_codes = Vec::with_capacity(seg_count);
                    for _ in 0..seg_count {
                        start_codes.push(sp.u16()?);
                    }
                    let mut id_deltas = Vec::with_capacity(seg_count);
                    for _ in 0..seg_count {
                        id_deltas.push(sp.i16()?);
                    }
                    let mut id_range_offsets = Vec::with_capacity(seg_count);
                    for _ in 0..seg_count {
                        id_range_offsets.push(sp.u16()?);
                    }
                    let mut segments = Vec::with_capacity(seg_count);
                    for i in 0..seg_count {
                        segments.push(Format4Segment {
                            end_code: end_codes[i],
                            start_code: start_codes[i],
                            id_delta: id_deltas[i],
                            id_range_offset: id_range_offsets[i],
                        });
                    }
                    subtables.push(CmapSubtable::Format4 { language, segments });
                }
                12 => {
                    sp.advance(2); // reserved
                    let _length = sp.u32()?;
                    let language = sp.u32()?;
                    let num_groups = sp.u32()?;
                    let mut groups = Vec::with_capacity(num_groups as usize);
                    for _ in 0..num_groups {
                        groups.push(SequentialMapGroup {
                            start_char_code: sp.u32()?,
                            end_char_code: sp.u32()?,
                            start_glyph_id: sp.u32()?,
                        });
                    }
                    subtables.push(CmapSubtable::Format12 { language, groups });
                }
                _ => return Err(FontError::UnsupportedCmapFormat(format)),
            }
        }
        Ok(Cmap {
            version,
            num_tables,
            records,
            subtables,
        })
    }

    fn write(&self, w: &mut Writer) -> Result<(), FontError> {
        // Compute subtable data first to know sizes and offsets
        let mut subtable_writers: Vec<Vec<u8>> = Vec::with_capacity(self.subtables.len());
        for subtable in &self.subtables {
            let mut sw = Writer::new();
            match subtable {
                CmapSubtable::Format0 { language, glyph_id_array } => {
                    sw.write_u16(0);
                    sw.write_u16(262);
                    sw.write_u16(*language);
                    for &b in glyph_id_array.iter() {
                        sw.write_u8(b);
                    }
                }
                CmapSubtable::Format4 { language, segments } => {
                    let seg_count = segments.len() as u16;
                    let length = 16 + seg_count * 8;
                    sw.write_u16(4);
                    sw.write_u16(length);
                    sw.write_u16(*language);
                    sw.write_u16(seg_count * 2);
                    sw.write_u16(0);
                    sw.write_u16(0);
                    sw.write_u16(0);
                    for seg in segments {
                        sw.write_u16(seg.end_code);
                    }
                    sw.write_u16(0); // reservedPad
                    for seg in segments {
                        sw.write_u16(seg.start_code);
                    }
                    for seg in segments {
                        sw.write_i16(seg.id_delta);
                    }
                    for seg in segments {
                        sw.write_u16(seg.id_range_offset);
                    }
                }
                CmapSubtable::Format12 { language, groups } => {
                    let length = 16 + groups.len() as u32 * 12;
                    sw.write_u16(12);
                    sw.write_u16(0); // reserved
                    sw.write_u32(length);
                    sw.write_u32(*language);
                    sw.write_u32(groups.len() as u32);
                    for g in groups {
                        sw.write_u32(g.start_char_code);
                        sw.write_u32(g.end_char_code);
                        sw.write_u32(g.start_glyph_id);
                    }
                }
            }
            sw.pad_to_4();
            subtable_writers.push(sw.into_vec());
        }

        let header_size = 4 + self.records.len() * 8;
        let mut offsets: Vec<u32> = Vec::with_capacity(self.subtables.len());
        let mut current_offset = header_size as u32;
        for data in &subtable_writers {
            offsets.push(current_offset);
            current_offset += data.len() as u32;
        }

        w.write_u16(self.version);
        w.write_u16(self.records.len() as u16);
        for (i, rec) in self.records.iter().enumerate() {
            w.write_u16(rec.platform_id);
            w.write_u16(rec.encoding_id);
            w.write_u32(offsets[i]);
        }
        for data in &subtable_writers {
            w.write_bytes(data);
        }
        Ok(())
    }
}
