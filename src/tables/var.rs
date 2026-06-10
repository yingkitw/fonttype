use crate::error::FontError;
use crate::parse::Parser;
use crate::write::Writer;

/// HVAR — Horizontal Metrics Variations table.
/// Stores variation data for horizontal metrics (advance widths and side bearings).
#[derive(Debug, Clone, PartialEq)]
pub struct Hvar {
    pub major_version: u16,
    pub minor_version: u16,
    pub raw: Vec<u8>,
}

impl Hvar {
    pub fn tag() -> crate::error::Tag {
        crate::error::Tag::new(b"HVAR")
    }

    pub fn parse(data: &[u8]) -> Result<Self, FontError> {
        let mut p = Parser::new(data, 0);
        let major_version = p.u16()?;
        let minor_version = p.u16()?;
        Ok(Hvar {
            major_version,
            minor_version,
            raw: data.to_vec(),
        })
    }

    pub fn write(&self, writer: &mut Writer) -> Result<(), FontError> {
        writer.write_bytes(&self.raw);
        Ok(())
    }
}

/// GVAR — Glyph Variations table.
/// Stores variation data for glyph outlines.
#[derive(Debug, Clone, PartialEq)]
pub struct Gvar {
    pub major_version: u16,
    pub minor_version: u16,
    pub raw: Vec<u8>,
}

impl Gvar {
    pub fn tag() -> crate::error::Tag {
        crate::error::Tag::new(b"gvar")
    }

    pub fn parse(data: &[u8]) -> Result<Self, FontError> {
        let mut p = Parser::new(data, 0);
        let major_version = p.u16()?;
        let minor_version = p.u16()?;
        Ok(Gvar {
            major_version,
            minor_version,
            raw: data.to_vec(),
        })
    }

    pub fn write(&self, writer: &mut Writer) -> Result<(), FontError> {
        writer.write_bytes(&self.raw);
        Ok(())
    }
}
