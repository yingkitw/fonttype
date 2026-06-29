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
    Format6 {
        language: u16,
        first_code: u16,
        glyph_id_array: Vec<u16>,
    },
    Format10 {
        language: u32,
        start_char_code: u32,
        glyph_id_array: Vec<u16>,
    },
    Format13 {
        language: u32,
        groups: Vec<ConstantMapGroup>,
    },
    Format14 {
        records: Vec<VariationSelectorRecord>,
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

#[derive(Debug, Clone, PartialEq)]
pub struct ConstantMapGroup {
    pub start_char_code: u32,
    pub end_char_code: u32,
    pub glyph_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariationSelectorRecord {
    pub var_selector: u32,
    pub default_uvs: Vec<UnicodeRange>,
    pub non_default_uvs: Vec<NonDefaultUvMapping>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnicodeRange {
    pub start_unicode_value: u32,
    pub additional_count: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NonDefaultUvMapping {
    pub unicode_value: u32,
    pub glyph_id: u16,
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
            CmapSubtable::Format6 { first_code, glyph_id_array, .. } => {
                let fc = *first_code as u32;
                if codepoint >= fc {
                    let idx = (codepoint - fc) as usize;
                    if idx < glyph_id_array.len() {
                        return Some(glyph_id_array[idx]);
                    }
                }
                None
            }
            CmapSubtable::Format10 { start_char_code, glyph_id_array, .. } => {
                if codepoint >= *start_char_code {
                    let idx = (codepoint - *start_char_code) as usize;
                    if idx < glyph_id_array.len() {
                        return Some(glyph_id_array[idx]);
                    }
                }
                None
            }
            CmapSubtable::Format13 { groups, .. } => {
                for g in groups {
                    if codepoint >= g.start_char_code && codepoint <= g.end_char_code {
                        return Some(g.glyph_id as u16);
                    }
                }
                None
            }
            CmapSubtable::Format14 { .. } => {
                // Format 14 maps (base char + variation selector) -> glyph;
                // single codepoint lookup is not applicable.
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
            CmapSubtable::Format6 { first_code, glyph_id_array, .. } => {
                glyph_id_array.iter().enumerate()
                    .filter(|(_, g)| **g == glyph_id)
                    .map(|(i, _)| *first_code as u32 + i as u32)
                    .collect()
            }
            CmapSubtable::Format10 { start_char_code, glyph_id_array, .. } => {
                glyph_id_array.iter().enumerate()
                    .filter(|(_, g)| **g == glyph_id)
                    .map(|(i, _)| *start_char_code + i as u32)
                    .collect()
            }
            CmapSubtable::Format13 { groups, .. } => {
                let mut cps = Vec::new();
                for g in groups {
                    if g.glyph_id == glyph_id as u32 {
                        for cp in g.start_char_code..=g.end_char_code {
                            cps.push(cp);
                        }
                    }
                }
                cps
            }
            CmapSubtable::Format14 { .. } => Vec::new(),
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
                6 => {
                    let _length = sp.u16()?;
                    let language = sp.u16()?;
                    let first_code = sp.u16()?;
                    let entry_count = sp.u16()?;
                    let mut glyph_id_array = Vec::with_capacity(entry_count as usize);
                    for _ in 0..entry_count {
                        glyph_id_array.push(sp.u16()?);
                    }
                    subtables.push(CmapSubtable::Format6 { language, first_code, glyph_id_array });
                }
                10 => {
                    sp.advance(2); // reserved
                    let _length = sp.u32()?;
                    let language = sp.u32()?;
                    let start_char_code = sp.u32()?;
                    let num_chars = sp.u32()?;
                    let mut glyph_id_array = Vec::with_capacity(num_chars as usize);
                    for _ in 0..num_chars {
                        glyph_id_array.push(sp.u16()?);
                    }
                    subtables.push(CmapSubtable::Format10 { language, start_char_code, glyph_id_array });
                }
                13 => {
                    sp.advance(2); // reserved
                    let _length = sp.u32()?;
                    let language = sp.u32()?;
                    let num_groups = sp.u32()?;
                    let mut groups = Vec::with_capacity(num_groups as usize);
                    for _ in 0..num_groups {
                        groups.push(ConstantMapGroup {
                            start_char_code: sp.u32()?,
                            end_char_code: sp.u32()?,
                            glyph_id: sp.u32()?,
                        });
                    }
                    subtables.push(CmapSubtable::Format13 { language, groups });
                }
                14 => {
                    let _length = sp.u32()?;
                    let num_records = sp.u32()?;
                    let subtable_start = offset + rec.subtable_offset as usize;
                    let mut records = Vec::with_capacity(num_records as usize);
                    for _ in 0..num_records {
                        let var_selector = sp.u24()?;
                        let default_uvs_offset = sp.u32()?;
                        let non_default_uvs_offset = sp.u32()?;
                        let mut default_uvs = Vec::new();
                        if default_uvs_offset != 0 {
                            let mut dp = Parser::new(buf, subtable_start + default_uvs_offset as usize);
                            let num_ranges = dp.u32()?;
                            for _ in 0..num_ranges {
                                default_uvs.push(UnicodeRange {
                                    start_unicode_value: dp.u24()?,
                                    additional_count: dp.u8()?,
                                });
                            }
                        }
                        let mut non_default_uvs = Vec::new();
                        if non_default_uvs_offset != 0 {
                            let mut ndp = Parser::new(buf, subtable_start + non_default_uvs_offset as usize);
                            let num_values = ndp.u32()?;
                            for _ in 0..num_values {
                                non_default_uvs.push(NonDefaultUvMapping {
                                    unicode_value: ndp.u24()?,
                                    glyph_id: ndp.u16()?,
                                });
                            }
                        }
                        records.push(VariationSelectorRecord { var_selector, default_uvs, non_default_uvs });
                    }
                    subtables.push(CmapSubtable::Format14 { records });
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
                CmapSubtable::Format6 { language, first_code, glyph_id_array } => {
                    let length = 10 + glyph_id_array.len() as u16 * 2;
                    sw.write_u16(6);
                    sw.write_u16(length);
                    sw.write_u16(*language);
                    sw.write_u16(*first_code);
                    sw.write_u16(glyph_id_array.len() as u16);
                    for &gid in glyph_id_array {
                        sw.write_u16(gid);
                    }
                }
                CmapSubtable::Format10 { language, start_char_code, glyph_id_array } => {
                    let length = 20 + glyph_id_array.len() as u32 * 2;
                    sw.write_u16(10);
                    sw.write_u16(0); // reserved
                    sw.write_u32(length);
                    sw.write_u32(*language);
                    sw.write_u32(*start_char_code);
                    sw.write_u32(glyph_id_array.len() as u32);
                    for &gid in glyph_id_array {
                        sw.write_u16(gid);
                    }
                }
                CmapSubtable::Format13 { language, groups } => {
                    let length = 16 + groups.len() as u32 * 12;
                    sw.write_u16(13);
                    sw.write_u16(0); // reserved
                    sw.write_u32(length);
                    sw.write_u32(*language);
                    sw.write_u32(groups.len() as u32);
                    for g in groups {
                        sw.write_u32(g.start_char_code);
                        sw.write_u32(g.end_char_code);
                        sw.write_u32(g.glyph_id);
                    }
                }
                CmapSubtable::Format14 { records } => {
                    let header_size = 10u32;
                    let record_size = 11u32;
                    let after_records = header_size + records.len() as u32 * record_size;

                    // Pre-build default and non-default UVS data
                    let mut default_data: Vec<Vec<u8>> = Vec::new();
                    let mut non_default_data: Vec<Vec<u8>> = Vec::new();
                    for rec in records {
                        if !rec.default_uvs.is_empty() {
                            let mut dw = Writer::new();
                            dw.write_u32(rec.default_uvs.len() as u32);
                            for r in &rec.default_uvs {
                                dw.write_u24(r.start_unicode_value);
                                dw.write_u8(r.additional_count);
                            }
                            default_data.push(dw.into_vec());
                        } else {
                            default_data.push(Vec::new());
                        }

                        if !rec.non_default_uvs.is_empty() {
                            let mut ndw = Writer::new();
                            ndw.write_u32(rec.non_default_uvs.len() as u32);
                            for m in &rec.non_default_uvs {
                                ndw.write_u24(m.unicode_value);
                                ndw.write_u16(m.glyph_id);
                            }
                            non_default_data.push(ndw.into_vec());
                        } else {
                            non_default_data.push(Vec::new());
                        }
                    }

                    // Compute offsets
                    let mut default_offsets = Vec::new();
                    let mut current_offset = after_records;
                    for (i, rec) in records.iter().enumerate() {
                        if rec.default_uvs.is_empty() {
                            default_offsets.push(0u32);
                        } else {
                            default_offsets.push(current_offset);
                            current_offset += default_data[i].len() as u32;
                        }
                    }
                    let mut non_default_offsets = Vec::new();
                    for (i, rec) in records.iter().enumerate() {
                        if rec.non_default_uvs.is_empty() {
                            non_default_offsets.push(0u32);
                        } else {
                            non_default_offsets.push(current_offset);
                            current_offset += non_default_data[i].len() as u32;
                        }
                    }

                    sw.write_u16(14);
                    sw.write_u32(current_offset); // total length
                    sw.write_u32(records.len() as u32);
                    for (i, rec) in records.iter().enumerate() {
                        sw.write_u24(rec.var_selector);
                        sw.write_u32(default_offsets[i]);
                        sw.write_u32(non_default_offsets[i]);
                    }
                    for data in &default_data {
                        sw.write_bytes(data);
                    }
                    for data in &non_default_data {
                        sw.write_bytes(data);
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
