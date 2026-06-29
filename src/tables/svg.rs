use crate::error::{FontError, Tag};
use crate::parse::Parser;
use crate::tables::Table;
use crate::write::Writer;

/// SVG table — SVG documents for color glyphs.
#[derive(Debug, Clone, PartialEq)]
pub struct Svg {
    pub version: u16,
    pub entries: Vec<SvgDocumentRecord>,
    pub documents: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SvgDocumentRecord {
    pub start_glyph_id: u16,
    pub end_glyph_id: u16,
    pub svg_doc_offset: u32,
    pub svg_doc_length: u32,
}

impl Table for Svg {
    fn tag() -> Tag {
        Tag::new(b"SVG ")
    }

    fn parse(buf: &[u8], offset: usize) -> Result<Self, FontError> {
        let mut p = Parser::new(buf, offset);
        let version = p.u16()?;
        let offset_doc_list = p.u32()? as usize;

        let mut dp = Parser::new(buf, offset + offset_doc_list);
        let num_entries = dp.u16()?;
        let mut entries = Vec::with_capacity(num_entries as usize);
        for _ in 0..num_entries {
            entries.push(SvgDocumentRecord {
                start_glyph_id: dp.u16()?,
                end_glyph_id: dp.u16()?,
                svg_doc_offset: dp.u32()?,
                svg_doc_length: dp.u32()?,
            });
        }

        let mut documents = Vec::with_capacity(num_entries as usize);
        for rec in &entries {
            let start = offset + offset_doc_list + rec.svg_doc_offset as usize;
            let end = start + rec.svg_doc_length as usize;
            if end > buf.len() {
                return Err(FontError::OutOfBounds {
                    offset: start,
                    length: rec.svg_doc_length as usize,
                    buf_len: buf.len(),
                });
            }
            documents.push(buf[start..end].to_vec());
        }

        Ok(Svg {
            version,
            entries,
            documents,
        })
    }

    fn write(&self, w: &mut Writer) -> Result<(), FontError> {
        let header_size = 6u32;
        let record_size = 12u32;
        let doc_list_offset = header_size;
        let first_doc_offset = 2u32 + self.entries.len() as u32 * record_size;

        w.write_u16(self.version);
        w.write_u32(doc_list_offset);

        w.write_u16(self.entries.len() as u16);
        let mut current_doc_offset = first_doc_offset;
        for (i, rec) in self.entries.iter().enumerate() {
            let doc_len = self.documents.get(i).map(|d| d.len() as u32).unwrap_or(0);
            w.write_u16(rec.start_glyph_id);
            w.write_u16(rec.end_glyph_id);
            w.write_u32(current_doc_offset);
            w.write_u32(doc_len);
            current_doc_offset += doc_len;
        }
        for doc in &self.documents {
            w.write_bytes(doc);
        }
        Ok(())
    }
}
