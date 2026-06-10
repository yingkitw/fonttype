use crate::error::{FontError, Tag};
use crate::parse::Parser;
use crate::tables::Table;
use crate::write::Writer;

#[derive(Debug, Clone, PartialEq)]
pub struct Post {
    pub version: i32, // Fixed
    pub italic_angle: i32,
    pub underline_position: i16,
    pub underline_thickness: i16,
    pub is_fixed_pitch: u32,
    pub min_mem_type42: u32,
    pub max_mem_type42: u32,
    pub min_mem_type1: u32,
    pub max_mem_type1: u32,
    pub names: Option<Vec<String>>,
}

impl Table for Post {
    fn tag() -> Tag {
        Tag::new(b"post")
    }

    fn parse(buf: &[u8], offset: usize) -> Result<Self, FontError> {
        let mut p = Parser::new(buf, offset);
        let version = p.fixed()?;
        let italic_angle = p.fixed()?;
        let underline_position = p.i16()?;
        let underline_thickness = p.i16()?;
        let is_fixed_pitch = p.u32()?;
        let min_mem_type42 = p.u32()?;
        let max_mem_type42 = p.u32()?;
        let min_mem_type1 = p.u32()?;
        let max_mem_type1 = p.u32()?;

        let mut names: Option<Vec<String>> = None;
        if version == 0x00020000 {
            let num_glyphs = p.u16()?;
            let mut name_indices = Vec::with_capacity(num_glyphs as usize);
            for _ in 0..num_glyphs {
                name_indices.push(p.u16()?);
            }
            let mut strings = Vec::new();
            for idx in name_indices {
                if idx < 258 {
                    strings.push(format!("glyph{}", idx));
                } else {
                    let len = p.u8()? as usize;
                    let s = std::str::from_utf8(p.slice(len)?).unwrap_or("");
                    p.advance(len);
                    strings.push(s.to_string());
                }
            }
            names = Some(strings);
        }

        Ok(Post {
            version,
            italic_angle,
            underline_position,
            underline_thickness,
            is_fixed_pitch,
            min_mem_type42,
            max_mem_type42,
            min_mem_type1,
            max_mem_type1,
            names,
        })
    }

    fn write(&self, w: &mut Writer) -> Result<(), FontError> {
        w.write_fixed(self.version);
        w.write_fixed(self.italic_angle);
        w.write_i16(self.underline_position);
        w.write_i16(self.underline_thickness);
        w.write_u32(self.is_fixed_pitch);
        w.write_u32(self.min_mem_type42);
        w.write_u32(self.max_mem_type42);
        w.write_u32(self.min_mem_type1);
        w.write_u32(self.max_mem_type1);
        if let Some(names) = &self.names {
            w.write_u16(names.len() as u16);
            for (i, _) in names.iter().enumerate() {
                w.write_u16((258 + i) as u16);
            }
            for name in names {
                w.write_u8(name.len() as u8);
                w.write_bytes(name.as_bytes());
            }
        }
        Ok(())
    }
}
