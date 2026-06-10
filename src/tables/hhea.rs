use crate::error::{FontError, Tag};
use crate::parse::Parser;
use crate::tables::Table;
use crate::write::Writer;

#[derive(Debug, Clone, PartialEq)]
pub struct Hhea {
    pub major_version: u16,
    pub minor_version: u16,
    pub ascender: i16,
    pub descender: i16,
    pub line_gap: i16,
    pub advance_width_max: u16,
    pub min_left_side_bearing: i16,
    pub min_right_side_bearing: i16,
    pub x_max_extent: i16,
    pub caret_slope_rise: i16,
    pub caret_slope_run: i16,
    pub caret_offset: i16,
    pub reserved: [i16; 4],
    pub metric_data_format: i16,
    pub number_of_hmetrics: u16,
}

impl Table for Hhea {
    fn tag() -> Tag {
        Tag::new(b"hhea")
    }

    fn parse(buf: &[u8], offset: usize) -> Result<Self, FontError> {
        let mut p = Parser::new(buf, offset);
        Ok(Hhea {
            major_version: p.u16()?,
            minor_version: p.u16()?,
            ascender: p.i16()?,
            descender: p.i16()?,
            line_gap: p.i16()?,
            advance_width_max: p.u16()?,
            min_left_side_bearing: p.i16()?,
            min_right_side_bearing: p.i16()?,
            x_max_extent: p.i16()?,
            caret_slope_rise: p.i16()?,
            caret_slope_run: p.i16()?,
            caret_offset: p.i16()?,
            reserved: [p.i16()?, p.i16()?, p.i16()?, p.i16()?],
            metric_data_format: p.i16()?,
            number_of_hmetrics: p.u16()?,
        })
    }

    fn write(&self, w: &mut Writer) -> Result<(), FontError> {
        w.write_u16(self.major_version);
        w.write_u16(self.minor_version);
        w.write_i16(self.ascender);
        w.write_i16(self.descender);
        w.write_i16(self.line_gap);
        w.write_u16(self.advance_width_max);
        w.write_i16(self.min_left_side_bearing);
        w.write_i16(self.min_right_side_bearing);
        w.write_i16(self.x_max_extent);
        w.write_i16(self.caret_slope_rise);
        w.write_i16(self.caret_slope_run);
        w.write_i16(self.caret_offset);
        for v in &self.reserved {
            w.write_i16(*v);
        }
        w.write_i16(self.metric_data_format);
        w.write_u16(self.number_of_hmetrics);
        Ok(())
    }
}
