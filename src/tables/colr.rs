use crate::error::{FontError, Tag};
use crate::parse::Parser;
use crate::tables::Table;
use crate::write::Writer;

/// COLR table — color glyph layers (v0).
#[derive(Debug, Clone, PartialEq)]
pub struct Colr {
    pub version: u16,
    pub base_glyphs: Vec<BaseGlyphRecord>,
    pub layers: Vec<LayerRecord>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BaseGlyphRecord {
    pub glyph_id: u16,
    pub first_layer_index: u16,
    pub num_layers: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayerRecord {
    pub glyph_id: u16,
    pub palette_index: u16,
}

impl Table for Colr {
    fn tag() -> Tag {
        Tag::new(b"COLR")
    }

    fn parse(buf: &[u8], offset: usize) -> Result<Self, FontError> {
        let mut p = Parser::new(buf, offset);
        let version = p.u16()?;
        let num_base_glyphs = p.u16()?;
        let offset_base_records = p.u32()? as usize;
        let offset_layer_records = p.u32()? as usize;
        let num_layer_records = p.u16()?;

        let mut base_glyphs = Vec::with_capacity(num_base_glyphs as usize);
        let mut bp = Parser::new(buf, offset + offset_base_records);
        for _ in 0..num_base_glyphs {
            base_glyphs.push(BaseGlyphRecord {
                glyph_id: bp.u16()?,
                first_layer_index: bp.u16()?,
                num_layers: bp.u16()?,
            });
        }

        let mut layers = Vec::with_capacity(num_layer_records as usize);
        let mut lp = Parser::new(buf, offset + offset_layer_records);
        for _ in 0..num_layer_records {
            layers.push(LayerRecord {
                glyph_id: lp.u16()?,
                palette_index: lp.u16()?,
            });
        }

        Ok(Colr {
            version,
            base_glyphs,
            layers,
        })
    }

    fn write(&self, w: &mut Writer) -> Result<(), FontError> {
        let header_size = 14u32;
        let base_size = self.base_glyphs.len() as u32 * 6;
        let layer_offset = header_size + base_size;

        w.write_u16(self.version);
        w.write_u16(self.base_glyphs.len() as u16);
        w.write_u32(header_size);
        w.write_u32(layer_offset);
        w.write_u16(self.layers.len() as u16);

        for bg in &self.base_glyphs {
            w.write_u16(bg.glyph_id);
            w.write_u16(bg.first_layer_index);
            w.write_u16(bg.num_layers);
        }
        for lr in &self.layers {
            w.write_u16(lr.glyph_id);
            w.write_u16(lr.palette_index);
        }
        Ok(())
    }
}
