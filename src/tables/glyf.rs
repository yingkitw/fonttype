use crate::error::{FontError, Tag};
use crate::tables::Table;
use crate::write::Writer;

#[derive(Debug, Clone, PartialEq)]
pub struct GlyfTable {
    pub glyphs: Vec<Glyph>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Glyph {
    Empty,
    Simple(SimpleGlyph),
    Composite(CompositeGlyph),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimpleGlyph {
    pub number_of_contours: i16,
    pub x_min: i16,
    pub y_min: i16,
    pub x_max: i16,
    pub y_max: i16,
    pub end_pts_of_contours: Vec<u16>,
    pub instructions: Vec<u8>,
    pub flags: Vec<u8>,
    pub x_coordinates: Vec<i16>,
    pub y_coordinates: Vec<i16>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositeGlyph {
    pub x_min: i16,
    pub y_min: i16,
    pub x_max: i16,
    pub y_max: i16,
    pub components: Vec<CompositeComponent>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositeComponent {
    pub glyph_index: u16,
    pub flags: u16,
    pub argument1: i16,
    pub argument2: i16,
    pub transformation: Option<CompositeTransform>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompositeTransform {
    Scale(f32),
    XyScale(f32, f32),
    TwoByTwo(f32, f32, f32, f32),
}

impl Glyph {
    pub fn from_points(contours: Vec<Vec<(i16, i16)>>) -> Self {
        if contours.is_empty() {
            return Glyph::Empty;
        }
        let mut end_pts = Vec::new();
        let mut flags = Vec::new();
        let mut x_coords = Vec::new();
        let mut y_coords = Vec::new();
        let mut x_min = i16::MAX;
        let mut y_min = i16::MAX;
        let mut x_max = i16::MIN;
        let mut y_max = i16::MIN;
        let mut total = 0u16;

        for contour in &contours {
            for &(x, y) in contour {
                x_min = x_min.min(x);
                y_min = y_min.min(y);
                x_max = x_max.max(x);
                y_max = y_max.max(y);
            }
            total += contour.len() as u16;
            end_pts.push(total.saturating_sub(1));
            for _ in contour {
                flags.push(0x01); // on-curve, 2-byte x/y
                x_coords.push(x_coords.last().copied().unwrap_or(0));
                y_coords.push(y_coords.last().copied().unwrap_or(0));
            }
        }

        // Now fill actual coordinates using relative encoding
        let mut final_flags = Vec::new();
        let mut final_x = Vec::new();
        let mut final_y = Vec::new();
        let mut prev_x = 0i16;
        let mut prev_y = 0i16;

        for contour in &contours {
            for &(x, y) in contour {
                let dx = x - prev_x;
                let dy = y - prev_y;
                prev_x = x;
                prev_y = y;
                final_flags.push(0x01); // on curve, 2-byte x and y
                final_x.push(dx);
                final_y.push(dy);
            }
        }

        Glyph::Simple(SimpleGlyph {
            number_of_contours: contours.len() as i16,
            x_min,
            y_min,
            x_max,
            y_max,
            end_pts_of_contours: end_pts,
            instructions: Vec::new(),
            flags: final_flags,
            x_coordinates: final_x,
            y_coordinates: final_y,
        })
    }

    pub fn write(&self, w: &mut Writer) {
        match self {
            Glyph::Empty => {}
            Glyph::Simple(g) => g.write(w),
            Glyph::Composite(g) => g.write(w),
        }
    }
}

impl SimpleGlyph {
    pub fn write(&self, w: &mut Writer) {
        w.write_i16(self.number_of_contours);
        w.write_i16(self.x_min);
        w.write_i16(self.y_min);
        w.write_i16(self.x_max);
        w.write_i16(self.y_max);
        for &pt in &self.end_pts_of_contours {
            w.write_u16(pt);
        }
        w.write_u16(self.instructions.len() as u16);
        w.write_bytes(&self.instructions);
        for &flag in &self.flags {
            w.write_u8(flag);
        }
        // We always use 2-byte coordinates (no short vector compression)
        for &x in &self.x_coordinates {
            w.write_i16(x);
        }
        for &y in &self.y_coordinates {
            w.write_i16(y);
        }
    }
}

impl CompositeGlyph {
    pub fn write(&self, w: &mut Writer) {
        w.write_i16(-1); // composite marker
        w.write_i16(self.x_min);
        w.write_i16(self.y_min);
        w.write_i16(self.x_max);
        w.write_i16(self.y_max);
        for (i, comp) in self.components.iter().enumerate() {
            let mut flags = comp.flags;
            if i < self.components.len() - 1 {
                flags |= 0x0020; // MORE_COMPONENTS
            } else {
                flags &= !0x0020;
            }
            // Determine if args fit in bytes
            let args_fit_byte = comp.argument1 >= i8::MIN as i16
                && comp.argument1 <= i8::MAX as i16
                && comp.argument2 >= i8::MIN as i16
                && comp.argument2 <= i8::MAX as i16;
            if args_fit_byte {
                flags &= !0x0001; // clear ARG_1_AND_2_ARE_WORDS
            } else {
                flags |= 0x0001; // set ARG_1_AND_2_ARE_WORDS
            }
            w.write_u16(flags);
            w.write_u16(comp.glyph_index);
            if args_fit_byte {
                w.write_i8(comp.argument1 as i8);
                w.write_i8(comp.argument2 as i8);
            } else {
                w.write_i16(comp.argument1);
                w.write_i16(comp.argument2);
            }
            if let Some(ref t) = comp.transformation {
                match *t {
                    CompositeTransform::Scale(s) => {
                        w.write_i16((s * 16384.0) as i16);
                    }
                    CompositeTransform::XyScale(x, y) => {
                        w.write_i16((x * 16384.0) as i16);
                        w.write_i16((y * 16384.0) as i16);
                    }
                    CompositeTransform::TwoByTwo(a, b, c, d) => {
                        w.write_i16((a * 16384.0) as i16);
                        w.write_i16((b * 16384.0) as i16);
                        w.write_i16((c * 16384.0) as i16);
                        w.write_i16((d * 16384.0) as i16);
                    }
                }
            }
        }
    }
}

impl GlyfTable {
    pub fn parse(data: &[u8], loca: &super::loca::LocaTable, glyf_offset: usize) -> Result<Self, FontError> {
        let mut glyphs = Vec::with_capacity(loca.offsets.len().saturating_sub(1));
        for i in 0..loca.offsets.len().saturating_sub(1) {
            let start = loca.offsets[i] as usize;
            let end = loca.offsets[i + 1] as usize;
            if start == end {
                glyphs.push(Glyph::Empty);
                continue;
            }
            let slice = &data[glyf_offset + start..glyf_offset + end];
            if slice.len() < 10 {
                glyphs.push(Glyph::Empty);
                continue;
            }
            let num_contours = i16::from_be_bytes([slice[0], slice[1]]);
            if num_contours >= 0 {
                let x_min = i16::from_be_bytes([slice[2], slice[3]]);
                let y_min = i16::from_be_bytes([slice[4], slice[5]]);
                let x_max = i16::from_be_bytes([slice[6], slice[7]]);
                let y_max = i16::from_be_bytes([slice[8], slice[9]]);
                let mut offset = 10usize;
                if slice.len() < 10 + num_contours as usize * 2 + 2 {
                    glyphs.push(Glyph::Empty);
                    continue;
                }
                let mut end_pts = Vec::with_capacity(num_contours as usize);
                for _ in 0..num_contours {
                    if offset + 1 >= slice.len() { break; }
                    let pt = u16::from_be_bytes([slice[offset], slice[offset + 1]]);
                    end_pts.push(pt);
                    offset += 2;
                }
                if offset + 1 >= slice.len() {
                    glyphs.push(Glyph::Empty);
                    continue;
                }
                let instr_len = u16::from_be_bytes([slice[offset], slice[offset + 1]]);
                offset += 2;
                if offset + instr_len as usize > slice.len() {
                    glyphs.push(Glyph::Empty);
                    continue;
                }
                let instructions = slice[offset..offset + instr_len as usize].to_vec();
                offset += instr_len as usize;
                let total_pts = end_pts.last().copied().unwrap_or(0) as usize + 1;
                let mut flags = Vec::with_capacity(total_pts);
                let mut j = 0;
                while j < total_pts {
                    if offset >= slice.len() { break; }
                    let flag = slice[offset];
                    offset += 1;
                    flags.push(flag);
                    if flag & 0x08 != 0 {
                        if offset >= slice.len() { break; }
                        let repeat = slice[offset];
                        offset += 1;
                        for _ in 0..repeat {
                            flags.push(flag);
                        }
                        j += repeat as usize;
                    }
                    j += 1;
                }
                let mut x_coords = Vec::with_capacity(total_pts);
                for &flag in &flags {
                    if offset >= slice.len() { break; }
                    if flag & 0x02 != 0 {
                        // short vector
                        let v = slice[offset] as i16;
                        offset += 1;
                        let signed = if flag & 0x10 != 0 { v } else { -v };
                        x_coords.push(signed);
                    } else {
                        if flag & 0x10 != 0 {
                            x_coords.push(0);
                        } else {
                            if offset + 2 > slice.len() { break; }
                            let v = i16::from_be_bytes([slice[offset], slice[offset + 1]]);
                            offset += 2;
                            x_coords.push(v);
                        }
                    }
                }
                let mut y_coords = Vec::with_capacity(total_pts);
                for &flag in &flags {
                    if offset >= slice.len() { break; }
                    if flag & 0x04 != 0 {
                        let v = slice[offset] as i16;
                        offset += 1;
                        let signed = if flag & 0x20 != 0 { v } else { -v };
                        y_coords.push(signed);
                    } else {
                        if flag & 0x20 != 0 {
                            y_coords.push(0);
                        } else {
                            if offset + 2 > slice.len() { break; }
                            let v = i16::from_be_bytes([slice[offset], slice[offset + 1]]);
                            offset += 2;
                            y_coords.push(v);
                        }
                    }
                }
                glyphs.push(Glyph::Simple(SimpleGlyph {
                    number_of_contours: num_contours,
                    x_min,
                    y_min,
                    x_max,
                    y_max,
                    end_pts_of_contours: end_pts,
                    instructions,
                    flags,
                    x_coordinates: x_coords,
                    y_coordinates: y_coords,
                }));
            } else {
                let x_min = i16::from_be_bytes([slice[2], slice[3]]);
                let y_min = i16::from_be_bytes([slice[4], slice[5]]);
                let x_max = i16::from_be_bytes([slice[6], slice[7]]);
                let y_max = i16::from_be_bytes([slice[8], slice[9]]);
                let mut offset = 10usize;
                let mut components = Vec::new();
                let mut has_more = true;
                while has_more {
                    let flags = u16::from_be_bytes([slice[offset], slice[offset + 1]]);
                    offset += 2;
                    let glyph_index = u16::from_be_bytes([slice[offset], slice[offset + 1]]);
                    offset += 2;

                    let arg1: i16;
                    let arg2: i16;
                    if flags & 0x0001 != 0 {
                        arg1 = i16::from_be_bytes([slice[offset], slice[offset + 1]]);
                        offset += 2;
                        arg2 = i16::from_be_bytes([slice[offset], slice[offset + 1]]);
                        offset += 2;
                    } else {
                        arg1 = slice[offset] as i8 as i16;
                        offset += 1;
                        arg2 = slice[offset] as i8 as i16;
                        offset += 1;
                    }

                    let mut transformation = None;
                    if flags & 0x0008 != 0 {
                        // WE_HAVE_A_SCALE
                        let val = i16::from_be_bytes([slice[offset], slice[offset + 1]]);
                        offset += 2;
                        transformation = Some(CompositeTransform::Scale(val as f32 / 16384.0));
                    } else if flags & 0x0040 != 0 {
                        // WE_HAVE_AN_X_AND_Y_SCALE
                        let xscale = i16::from_be_bytes([slice[offset], slice[offset + 1]]);
                        offset += 2;
                        let yscale = i16::from_be_bytes([slice[offset], slice[offset + 1]]);
                        offset += 2;
                        transformation = Some(CompositeTransform::XyScale(
                            xscale as f32 / 16384.0,
                            yscale as f32 / 16384.0,
                        ));
                    } else if flags & 0x0080 != 0 {
                        // WE_HAVE_A_TWO_BY_TWO
                        let a = i16::from_be_bytes([slice[offset], slice[offset + 1]]);
                        offset += 2;
                        let b = i16::from_be_bytes([slice[offset], slice[offset + 1]]);
                        offset += 2;
                        let c = i16::from_be_bytes([slice[offset], slice[offset + 1]]);
                        offset += 2;
                        let d = i16::from_be_bytes([slice[offset], slice[offset + 1]]);
                        offset += 2;
                        transformation = Some(CompositeTransform::TwoByTwo(
                            a as f32 / 16384.0,
                            b as f32 / 16384.0,
                            c as f32 / 16384.0,
                            d as f32 / 16384.0,
                        ));
                    }

                    components.push(CompositeComponent {
                        glyph_index,
                        flags,
                        argument1: arg1,
                        argument2: arg2,
                        transformation,
                    });

                    has_more = flags & 0x0020 != 0;
                }
                glyphs.push(Glyph::Composite(CompositeGlyph {
                    x_min,
                    y_min,
                    x_max,
                    y_max,
                    components,
                }));
            }
        }
        Ok(GlyfTable { glyphs })
    }

    pub fn write(&self) -> Vec<u8> {
        let mut w = Writer::new();
        for glyph in &self.glyphs {
            glyph.write(&mut w);
        }
        w.into_vec()
    }
}

impl Table for GlyfTable {
    fn tag() -> Tag {
        Tag::new(b"glyf")
    }

    fn parse(_buf: &[u8], _offset: usize) -> Result<Self, FontError> {
        Err(FontError::invalid_table(
            Self::tag(),
            "glyf requires loca table for parsing; use GlyfTable::parse_with_loca",
        ))
    }

    fn write(&self, w: &mut Writer) -> Result<(), FontError> {
        for glyph in &self.glyphs {
            glyph.write(w);
        }
        Ok(())
    }
}
