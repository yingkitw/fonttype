use crate::error::FontError;
use crate::font::Font;
use crate::parse::Parser;

#[derive(Debug, Clone, PartialEq)]
pub struct Ttc {
    pub version: u32,
    pub num_fonts: u32,
    pub offsets: Vec<u32>,
    pub dsig_tag: Option<u32>,
    pub dsig_length: Option<u32>,
    pub dsig_offset: Option<u32>,
}

impl Ttc {
    pub fn parse(buf: &[u8]) -> Result<Self, FontError> {
        let mut p = Parser::new(buf, 0);
        let tag = p.u32()?;
        if tag != 0x74746366 {
            return Err(FontError::invalid_table(
                crate::error::Tag::new(b"ttcf"),
                &format!("Expected ttcTag 0x74746366, got 0x{:08X}", tag),
            ));
        }
        let version = p.u32()?;
        let num_fonts = p.u32()?;
        let mut offsets = Vec::with_capacity(num_fonts as usize);
        for _ in 0..num_fonts {
            offsets.push(p.u32()?);
        }
        let (dsig_tag, dsig_length, dsig_offset) = if version == 0x00020000 {
            (Some(p.u32()?), Some(p.u32()?), Some(p.u32()?))
        } else {
            (None, None, None)
        };
        Ok(Ttc {
            version,
            num_fonts,
            offsets,
            dsig_tag,
            dsig_length,
            dsig_offset,
        })
    }

    pub fn font_at(&self, buf: &[u8], index: usize) -> Result<Font, FontError> {
        if index >= self.offsets.len() {
            return Err(FontError::invalid_table(
                crate::error::Tag::new(b"ttcf"),
                &format!("Font index {} out of range ({} fonts)", index, self.num_fonts),
            ));
        }
        let offset = self.offsets[index] as usize;
        Font::read(&buf[offset..])
    }

    pub fn fonts<'a>(&self, buf: &'a [u8]) -> Vec<Result<Font, FontError>> {
        self.offsets
            .iter()
            .map(|&off| Font::read(&buf[off as usize..]))
            .collect()
    }
}
