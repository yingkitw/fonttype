use crate::error::{FontError, Tag};
use crate::parse::Parser;
use crate::tables::Table;
use crate::write::Writer;

#[derive(Debug, Clone, PartialEq)]
pub struct NameRecord {
    pub platform_id: u16,
    pub encoding_id: u16,
    pub language_id: u16,
    pub name_id: u16,
    pub string: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Name {
    pub format: u16,
    pub count: u16,
    pub string_offset: u16,
    pub records: Vec<NameRecord>,
}

impl Name {
    pub fn family_name(&self) -> Option<String> {
        self.find(1)
    }

    pub fn subfamily_name(&self) -> Option<String> {
        self.find(2)
    }

    pub fn full_name(&self) -> Option<String> {
        self.find(4)
    }

    pub fn version(&self) -> Option<String> {
        self.find(5)
    }

    fn find(&self, name_id: u16) -> Option<String> {
        self.records
            .iter()
            .find(|r| r.name_id == name_id)
            .map(|r| r.string.clone())
    }
}

impl Table for Name {
    fn tag() -> Tag {
        Tag::new(b"name")
    }

    fn parse(buf: &[u8], offset: usize) -> Result<Self, FontError> {
        let mut p = Parser::new(buf, offset);
        let format = p.u16()?;
        let count = p.u16()?;
        let string_offset = p.u16()?;
        let mut records = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let platform_id = p.u16()?;
            let encoding_id = p.u16()?;
            let language_id = p.u16()?;
            let name_id = p.u16()?;
            let length = p.u16()?;
            let string_offset_rel = p.u16()?;
            let abs_offset = offset + string_offset as usize + string_offset_rel as usize;
            let s = if platform_id == 3 || (platform_id == 0 && encoding_id == 3) {
                // UTF-16BE
                let bytes = Parser::new(buf, abs_offset).slice(length as usize)?;
                let u16s: Vec<u16> = bytes
                    .chunks_exact(2)
                    .map(|b| u16::from_be_bytes([b[0], b[1]]))
                    .collect();
                String::from_utf16(&u16s).unwrap_or_default()
            } else {
                let bytes = Parser::new(buf, abs_offset).slice(length as usize)?;
                String::from_utf8_lossy(bytes).to_string()
            };
            records.push(NameRecord {
                platform_id,
                encoding_id,
                language_id,
                name_id,
                string: s,
            });
        }
        Ok(Name {
            format,
            count,
            string_offset,
            records,
        })
    }

    fn write(&self, w: &mut Writer) -> Result<(), FontError> {
        // Two-pass: first compute encoded strings and their offsets
        let mut encoded: Vec<Vec<u8>> = Vec::with_capacity(self.records.len());
        let mut string_offsets: Vec<u16> = Vec::with_capacity(self.records.len());
        let header_size = 6 + self.records.len() * 12;
        let mut current_offset: u16 = 0;
        for rec in &self.records {
            let bytes: Vec<u8> = if rec.platform_id == 3 || (rec.platform_id == 0 && rec.encoding_id == 3) {
                rec.string
                    .encode_utf16()
                    .flat_map(|c| c.to_be_bytes())
                    .collect()
            } else {
                rec.string.as_bytes().to_vec()
            };
            string_offsets.push(current_offset);
            current_offset += bytes.len() as u16;
            encoded.push(bytes);
        }
        w.write_u16(self.format);
        w.write_u16(self.records.len() as u16);
        w.write_u16(header_size as u16);
        for (i, rec) in self.records.iter().enumerate() {
            w.write_u16(rec.platform_id);
            w.write_u16(rec.encoding_id);
            w.write_u16(rec.language_id);
            w.write_u16(rec.name_id);
            w.write_u16(encoded[i].len() as u16);
            w.write_u16(string_offsets[i]);
        }
        for bytes in &encoded {
            w.write_bytes(bytes);
        }
        Ok(())
    }
}
