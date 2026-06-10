use crate::error::{FontError, Tag};
use crate::parse::Parser;
use crate::tables::Table;
use crate::write::Writer;

#[derive(Debug, Clone, PartialEq)]
pub struct Os2 {
    pub version: u16,
    pub x_avg_char_width: i16,
    pub us_weight_class: u16,
    pub us_width_class: u16,
    pub fs_type: u16,
    pub y_subscript_x_size: i16,
    pub y_subscript_y_size: i16,
    pub y_subscript_x_offset: i16,
    pub y_subscript_y_offset: i16,
    pub y_superscript_x_size: i16,
    pub y_superscript_y_size: i16,
    pub y_superscript_x_offset: i16,
    pub y_superscript_y_offset: i16,
    pub y_strikeout_size: i16,
    pub y_strikeout_position: i16,
    pub s_family_class: i16,
    pub panose: [u8; 10],
    pub ul_unicode_range1: u32,
    pub ul_unicode_range2: u32,
    pub ul_unicode_range3: u32,
    pub ul_unicode_range4: u32,
    pub ach_vend_id: [u8; 4],
    pub fs_selection: u16,
    pub us_first_char_index: u16,
    pub us_last_char_index: u16,
    pub s_typo_ascender: i16,
    pub s_typo_descender: i16,
    pub s_typo_line_gap: i16,
    pub us_win_ascent: u16,
    pub us_win_descent: u16,
    pub ul_code_page_range1: Option<u32>,
    pub ul_code_page_range2: Option<u32>,
    pub sx_height: Option<i16>,
    pub s_cap_height: Option<i16>,
    pub us_default_char: Option<u16>,
    pub us_break_char: Option<u16>,
    pub us_max_context: Option<u16>,
    pub us_lower_optical_point_size: Option<u16>,
    pub us_upper_optical_point_size: Option<u16>,
}

impl Table for Os2 {
    fn tag() -> Tag {
        Tag::new(b"OS/2")
    }

    fn parse(buf: &[u8], offset: usize) -> Result<Self, FontError> {
        let mut p = Parser::new(buf, offset);
        let version = p.u16()?;
        let mut os2 = Os2 {
            version,
            x_avg_char_width: p.i16()?,
            us_weight_class: p.u16()?,
            us_width_class: p.u16()?,
            fs_type: p.u16()?,
            y_subscript_x_size: p.i16()?,
            y_subscript_y_size: p.i16()?,
            y_subscript_x_offset: p.i16()?,
            y_subscript_y_offset: p.i16()?,
            y_superscript_x_size: p.i16()?,
            y_superscript_y_size: p.i16()?,
            y_superscript_x_offset: p.i16()?,
            y_superscript_y_offset: p.i16()?,
            y_strikeout_size: p.i16()?,
            y_strikeout_position: p.i16()?,
            s_family_class: p.i16()?,
            panose: {
                let b = p.slice(10)?;
                p.advance(10);
                [b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9]]
            },
            ul_unicode_range1: p.u32()?,
            ul_unicode_range2: p.u32()?,
            ul_unicode_range3: p.u32()?,
            ul_unicode_range4: p.u32()?,
            ach_vend_id: {
                let b = p.slice(4)?;
                p.advance(4);
                [b[0], b[1], b[2], b[3]]
            },
            fs_selection: p.u16()?,
            us_first_char_index: p.u16()?,
            us_last_char_index: p.u16()?,
            s_typo_ascender: p.i16()?,
            s_typo_descender: p.i16()?,
            s_typo_line_gap: p.i16()?,
            us_win_ascent: p.u16()?,
            us_win_descent: p.u16()?,
            ul_code_page_range1: None,
            ul_code_page_range2: None,
            sx_height: None,
            s_cap_height: None,
            us_default_char: None,
            us_break_char: None,
            us_max_context: None,
            us_lower_optical_point_size: None,
            us_upper_optical_point_size: None,
        };
        if version >= 1 {
            os2.ul_code_page_range1 = Some(p.u32()?);
            os2.ul_code_page_range2 = Some(p.u32()?);
        }
        if version >= 2 {
            os2.sx_height = Some(p.i16()?);
            os2.s_cap_height = Some(p.i16()?);
            os2.us_default_char = Some(p.u16()?);
            os2.us_break_char = Some(p.u16()?);
            os2.us_max_context = Some(p.u16()?);
        }
        if version >= 5 {
            os2.us_lower_optical_point_size = Some(p.u16()?);
            os2.us_upper_optical_point_size = Some(p.u16()?);
        }
        Ok(os2)
    }

    fn write(&self, w: &mut Writer) -> Result<(), FontError> {
        w.write_u16(self.version);
        w.write_i16(self.x_avg_char_width);
        w.write_u16(self.us_weight_class);
        w.write_u16(self.us_width_class);
        w.write_u16(self.fs_type);
        w.write_i16(self.y_subscript_x_size);
        w.write_i16(self.y_subscript_y_size);
        w.write_i16(self.y_subscript_x_offset);
        w.write_i16(self.y_subscript_y_offset);
        w.write_i16(self.y_superscript_x_size);
        w.write_i16(self.y_superscript_y_size);
        w.write_i16(self.y_superscript_x_offset);
        w.write_i16(self.y_superscript_y_offset);
        w.write_i16(self.y_strikeout_size);
        w.write_i16(self.y_strikeout_position);
        w.write_i16(self.s_family_class);
        w.write_bytes(&self.panose);
        w.write_u32(self.ul_unicode_range1);
        w.write_u32(self.ul_unicode_range2);
        w.write_u32(self.ul_unicode_range3);
        w.write_u32(self.ul_unicode_range4);
        w.write_bytes(&self.ach_vend_id);
        w.write_u16(self.fs_selection);
        w.write_u16(self.us_first_char_index);
        w.write_u16(self.us_last_char_index);
        w.write_i16(self.s_typo_ascender);
        w.write_i16(self.s_typo_descender);
        w.write_i16(self.s_typo_line_gap);
        w.write_u16(self.us_win_ascent);
        w.write_u16(self.us_win_descent);
        if self.version >= 1 {
            w.write_u32(self.ul_code_page_range1.unwrap_or(0));
            w.write_u32(self.ul_code_page_range2.unwrap_or(0));
        }
        if self.version >= 2 {
            w.write_i16(self.sx_height.unwrap_or(0));
            w.write_i16(self.s_cap_height.unwrap_or(0));
            w.write_u16(self.us_default_char.unwrap_or(0));
            w.write_u16(self.us_break_char.unwrap_or(0));
            w.write_u16(self.us_max_context.unwrap_or(0));
        }
        if self.version >= 5 {
            w.write_u16(self.us_lower_optical_point_size.unwrap_or(0));
            w.write_u16(self.us_upper_optical_point_size.unwrap_or(0xFFFF));
        }
        Ok(())
    }
}
