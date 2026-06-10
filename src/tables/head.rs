use crate::error::{FontError, Tag};
use crate::parse::Parser;
use crate::tables::Table;
use crate::write::Writer;

#[derive(Debug, Clone, PartialEq)]
pub struct Head {
    pub major_version: u16,
    pub minor_version: u16,
    pub font_revision: i32, // Fixed
    pub check_sum_adjustment: u32,
    pub magic_number: u32,
    pub flags: u16,
    pub units_per_em: u16,
    pub created: i64,       // longDateTime
    pub modified: i64,      // longDateTime
    pub x_min: i16,
    pub y_min: i16,
    pub x_max: i16,
    pub y_max: i16,
    pub mac_style: u16,
    pub lowest_rec_ppem: u16,
    pub font_direction_hint: i16,
    pub index_to_loc_format: i16,
    pub glyph_data_format: i16,
}

impl Table for Head {
    fn tag() -> Tag {
        Tag::new(b"head")
    }

    fn parse(buf: &[u8], offset: usize) -> Result<Self, FontError> {
        let mut p = Parser::new(buf, offset);
        Ok(Head {
            major_version: p.u16()?,
            minor_version: p.u16()?,
            font_revision: p.fixed()?,
            check_sum_adjustment: p.u32()?,
            magic_number: p.u32()?,
            flags: p.u16()?,
            units_per_em: p.u16()?,
            created: p.longdatetime()?,
            modified: p.longdatetime()?,
            x_min: p.i16()?,
            y_min: p.i16()?,
            x_max: p.i16()?,
            y_max: p.i16()?,
            mac_style: p.u16()?,
            lowest_rec_ppem: p.u16()?,
            font_direction_hint: p.i16()?,
            index_to_loc_format: p.i16()?,
            glyph_data_format: p.i16()?,
        })
    }

    fn write(&self, w: &mut Writer) -> Result<(), FontError> {
        w.write_u16(self.major_version);
        w.write_u16(self.minor_version);
        w.write_fixed(self.font_revision);
        w.write_u32(self.check_sum_adjustment);
        w.write_u32(self.magic_number);
        w.write_u16(self.flags);
        w.write_u16(self.units_per_em);
        w.write_longdatetime(self.created);
        w.write_longdatetime(self.modified);
        w.write_i16(self.x_min);
        w.write_i16(self.y_min);
        w.write_i16(self.x_max);
        w.write_i16(self.y_max);
        w.write_u16(self.mac_style);
        w.write_u16(self.lowest_rec_ppem);
        w.write_i16(self.font_direction_hint);
        w.write_i16(self.index_to_loc_format);
        w.write_i16(self.glyph_data_format);
        Ok(())
    }
}
