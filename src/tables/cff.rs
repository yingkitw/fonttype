use crate::error::{FontError, Tag};
use crate::parse::Parser;
use crate::tables::Table;
use crate::write::Writer;

/// CFF / CFF2 table — PostScript outlines.
/// For round-trip fidelity the full raw bytes are preserved.
/// Top-level INDEXes and the Top DICT are parsed for inspection.
#[derive(Debug, Clone, PartialEq)]
pub struct Cff {
    pub is_cff2: bool,
    pub major_version: u8,
    pub minor_version: u8,
    pub header_size: u8,
    pub off_size: u8,
    pub name_index: Vec<String>,
    pub top_dict: Vec<TopDictEntry>,
    pub raw: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TopDictEntry {
    pub operator: u16,
    pub operands: Vec<DictOperand>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DictOperand {
    Integer(i32),
    Real(f64),
}

impl Cff {
    pub fn parse(data: &[u8]) -> Result<Self, FontError> {
        let raw = data.to_vec();
        let mut p = Parser::new(data, 0);
        let major_version = p.u8()?;
        let minor_version = p.u8()?;
        let is_cff2 = major_version == 2;
        let header_size = p.u8()?;
        let off_size = if is_cff2 { 0 } else { p.u8()? };

        let mut name_index = Vec::new();
        let mut top_dict = Vec::new();

        if !is_cff2 {
            // CFF 1.0: Name INDEX follows header
            let name_idx_offset = header_size as usize;
            if let Ok(names) = parse_index(data, name_idx_offset) {
                name_index = names.into_iter().map(|b| String::from_utf8_lossy(b).to_string()).collect();
                // Top DICT INDEX follows Name INDEX
                let top_idx_offset = index_end(data, name_idx_offset);
                if let Ok(dict_data) = parse_index(data, top_idx_offset) {
                    if let Some(td) = dict_data.into_iter().next() {
                        top_dict = parse_dict(&td);
                    }
                }
            }
        } else {
            // CFF2: GlobalSubr INDEX offset is at headerSize
            // Top DICT data starts at headerSize + 5 (topDictLength is 2 bytes)
            // For simplicity we skip deep CFF2 parsing in v0.1
        }

        Ok(Cff {
            is_cff2,
            major_version,
            minor_version,
            header_size,
            off_size,
            name_index,
            top_dict,
            raw,
        })
    }
}

impl Table for Cff {
    fn tag() -> Tag {
        Tag::new(b"CFF ")
    }

    fn parse(buf: &[u8], offset: usize) -> Result<Self, FontError> {
        let data = Parser::new(buf, offset).slice(buf.len() - offset)?;
        Cff::parse(data)
    }

    fn write(&self, w: &mut Writer) -> Result<(), FontError> {
        w.write_bytes(&self.raw);
        Ok(())
    }
}

fn parse_index<'a>(buf: &'a [u8], offset: usize) -> Result<Vec<&'a [u8]>, FontError> {
    let mut p = Parser::new(buf, offset);
    let count = p.u16()? as usize;
    if count == 0 {
        return Ok(vec![]);
    }
    let off_size = p.u8()? as usize;
    let mut offsets = Vec::with_capacity(count + 1);
    for _ in 0..=count {
        let off = match off_size {
            1 => p.u8()? as usize,
            2 => p.u16()? as usize,
            4 => p.u32()? as usize,
            _ => return Err(FontError::invalid_table(Tag::new(b"CFF "), "Invalid offSize in INDEX")),
        };
        offsets.push(off);
    }
    let data_start = offset + 2 + 1 + (count + 1) * off_size;
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let start = data_start + offsets[i] - 1;
        let end = data_start + offsets[i + 1] - 1;
        entries.push(&buf[start..end]);
    }
    Ok(entries)
}

fn index_end(buf: &[u8], offset: usize) -> usize {
    let mut p = Parser::new(buf, offset);
    let count = p.u16().unwrap_or(0) as usize;
    if count == 0 {
        return offset + 2;
    }
    let off_size = p.u8().unwrap_or(1) as usize;
    let data_offset = offset + 2 + 1 + (count + 1) * off_size;
    let last_off = match off_size {
        1 => {
            p.advance(count * 1);
            p.u8().unwrap_or(0) as usize
        }
        2 => {
            p.advance(count * 2);
            p.u16().unwrap_or(0) as usize
        }
        4 => {
            p.advance(count * 4);
            p.u32().unwrap_or(0) as usize
        }
        _ => 0,
    };
    data_offset + last_off - 1
}

fn parse_dict(data: &[u8]) -> Vec<TopDictEntry> {
    let mut entries = Vec::new();
    let mut operands = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i];
        if b0 <= 21 {
            // Operator
            let op = if b0 == 12 {
                i += 1;
                if i >= data.len() { break; }
                1200 + data[i] as u16
            } else {
                b0 as u16
            };
            entries.push(TopDictEntry { operator: op, operands: std::mem::take(&mut operands) });
            i += 1;
        } else if b0 == 28 || b0 == 29 {
            // Integer
            if b0 == 28 && i + 2 < data.len() {
                let val = i16::from_be_bytes([data[i + 1], data[i + 2]]) as i32;
                operands.push(DictOperand::Integer(val));
                i += 3;
            } else if b0 == 29 && i + 4 < data.len() {
                let val = i32::from_be_bytes([data[i + 1], data[i + 2], data[i + 3], data[i + 4]]);
                operands.push(DictOperand::Integer(val));
                i += 5;
            } else {
                break;
            }
        } else if b0 >= 32 && b0 <= 246 {
            operands.push(DictOperand::Integer((b0 as i32) - 139));
            i += 1;
        } else if b0 >= 247 && b0 <= 250 {
            if i + 1 >= data.len() { break; }
            operands.push(DictOperand::Integer(((b0 as i32) - 247) * 256 + (data[i + 1] as i32) + 108));
            i += 2;
        } else if b0 >= 251 && b0 <= 254 {
            if i + 1 >= data.len() { break; }
            operands.push(DictOperand::Integer(-((b0 as i32) - 251) * 256 - (data[i + 1] as i32) - 108));
            i += 2;
        } else if b0 == 30 {
            // Real number — skip for basic parsing
            i += 1;
            while i < data.len() && data[i] & 0x0F != 0x0F {
                i += 1;
            }
            i += 1;
        } else {
            i += 1;
        }
    }
    entries
}
