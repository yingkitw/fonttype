use crate::error::{FontError, Tag};
use crate::parse::Parser;
use crate::tables::Table;
use crate::write::Writer;

/// CPAL table — color palettes.
#[derive(Debug, Clone, PartialEq)]
pub struct Cpal {
    pub version: u16,
    pub num_palette_entries: u16,
    pub palettes: Vec<Palette>,
    pub colors: Vec<ColorRecord>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    pub color_indices: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct ColorRecord {
    pub blue: u8,
    pub green: u8,
    pub red: u8,
    pub alpha: u8,
}

impl ColorRecord {
    pub fn to_rgba(&self) -> (u8, u8, u8, u8) {
        (self.red, self.green, self.blue, self.alpha)
    }
}

impl Table for Cpal {
    fn tag() -> Tag {
        Tag::new(b"CPAL")
    }

    fn parse(buf: &[u8], offset: usize) -> Result<Self, FontError> {
        let mut p = Parser::new(buf, offset);
        let version = p.u16()?;
        let num_palette_entries = p.u16()?;
        let num_palettes = p.u16()?;
        let num_color_records = p.u16()?;
        let offset_first_color = if version == 0 {
            p.u16()? as u32
        } else {
            p.u32()?
        };

        let mut palette_indices = Vec::with_capacity(num_palettes as usize);
        for _ in 0..num_palettes {
            palette_indices.push(p.u16()?);
        }

        let mut colors = Vec::with_capacity(num_color_records as usize);
        let mut cp = Parser::new(buf, offset + offset_first_color as usize);
        for _ in 0..num_color_records {
            colors.push(ColorRecord {
                blue: cp.u8()?,
                green: cp.u8()?,
                red: cp.u8()?,
                alpha: cp.u8()?,
            });
        }

        let mut palettes = Vec::with_capacity(num_palettes as usize);
        for idx in palette_indices {
            let start = idx as usize;
            let end = start + num_palette_entries as usize;
            if end > colors.len() {
                return Err(FontError::invalid_table(
                    Self::tag(),
                    format!("Palette index {} exceeds color records", idx),
                ));
            }
            let color_indices: Vec<u16> = (start..end).map(|i| i as u16).collect();
            palettes.push(Palette { color_indices });
        }

        Ok(Cpal {
            version,
            num_palette_entries,
            palettes,
            colors,
        })
    }

    fn write(&self, w: &mut Writer) -> Result<(), FontError> {
        let header_size = if self.version == 0 {
            10u32 + self.palettes.len() as u32 * 2
        } else {
            12u32 + self.palettes.len() as u32 * 2
        };
        let color_offset = header_size;

        w.write_u16(self.version);
        w.write_u16(self.num_palette_entries);
        w.write_u16(self.palettes.len() as u16);
        w.write_u16(self.colors.len() as u16);
        if self.version == 0 {
            w.write_u16(color_offset as u16);
        } else {
            w.write_u32(color_offset);
        }
        for palette in &self.palettes {
            if let Some(&first) = palette.color_indices.first() {
                w.write_u16(first);
            } else {
                w.write_u16(0);
            }
        }
        for c in &self.colors {
            w.write_u8(c.blue);
            w.write_u8(c.green);
            w.write_u8(c.red);
            w.write_u8(c.alpha);
        }
        Ok(())
    }
}
