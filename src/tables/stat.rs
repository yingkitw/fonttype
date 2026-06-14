use crate::error::{FontError, Tag};
use crate::parse::Parser;
use crate::tables::Table;
use crate::write::Writer;

#[derive(Debug, Clone, PartialEq)]
pub struct Stat {
    pub major_version: u16,
    pub minor_version: u16,
    pub design_axes: Vec<DesignAxisRecord>,
    pub axis_values: Vec<AxisValueRecord>,
    pub elided_fallback_name_id: Option<u16>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DesignAxisRecord {
    pub axis_tag: Tag,
    pub axis_name_id: u16,
    pub axis_ordering: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AxisValueRecord {
    pub format: u16,
    pub axis_index: u16,
    pub flags: u16,
    pub value_name_id: u16,
    pub value: f32,
    pub linked_value: Option<f32>,
    pub nominal_value: Option<f32>,
    pub range_min_value: Option<f32>,
    pub range_max_value: Option<f32>,
    pub axis_values: Vec<(u16, f32)>, // format 4: (axisIndex, value)
}

impl Stat {
    pub fn parse(data: &[u8]) -> Result<Self, FontError> {
        let mut p = Parser::new(data, 0);
        let major_version = p.u16()?;
        let minor_version = p.u16()?;
        let design_axis_size = p.u16()?;
        let design_axis_count = p.u16()?;
        let design_axis_offset = p.u32()?;
        let axis_value_count = p.u16()?;
        let axis_value_offset = p.u32()?;

        let elided_fallback_name_id = if major_version == 1 && minor_version >= 1 {
            Some(p.u16()?)
        } else {
            None
        };

        // Header sizes: v1.0 = 18 bytes, v1.1+ = 20 bytes

        let mut design_axes = Vec::with_capacity(design_axis_count as usize);
        for i in 0..design_axis_count {
            let off = design_axis_offset as usize + i as usize * design_axis_size as usize;
            let mut ap = Parser::new(data, off);
            let axis_tag = ap.tag()?;
            let axis_name_id = ap.u16()?;
            let axis_ordering = ap.u16()?;
            design_axes.push(DesignAxisRecord {
                axis_tag,
                axis_name_id,
                axis_ordering,
            });
        }

        let mut axis_values = Vec::with_capacity(axis_value_count as usize);
        if axis_value_count > 0 {
            let mut vp = Parser::new(data, axis_value_offset as usize);
            let _array_format = vp.u16()?; // always 1
            let count = vp.u16()?;
            let mut offsets = Vec::with_capacity(count as usize);
            for _ in 0..count {
                offsets.push(vp.u16()?);
            }
            for off in offsets {
                let mut avp = Parser::new(data, off as usize);
                let format = avp.u16()?;
                let axis_index = avp.u16()?;
                let flags = avp.u16()?;
                let value_name_id = avp.u16()?;
                let mut rec = AxisValueRecord {
                    format,
                    axis_index,
                    flags,
                    value_name_id,
                    value: 0.0,
                    linked_value: None,
                    nominal_value: None,
                    range_min_value: None,
                    range_max_value: None,
                    axis_values: Vec::new(),
                };
                match format {
                    1 => {
                        rec.value = avp.fixed()? as f32 / 65536.0;
                    }
                    2 => {
                        rec.nominal_value = Some(avp.fixed()? as f32 / 65536.0);
                        rec.range_min_value = Some(avp.fixed()? as f32 / 65536.0);
                        rec.range_max_value = Some(avp.fixed()? as f32 / 65536.0);
                    }
                    3 => {
                        rec.value = avp.fixed()? as f32 / 65536.0;
                        rec.linked_value = Some(avp.fixed()? as f32 / 65536.0);
                    }
                    4 => {
                        let axis_count = avp.u16()?;
                        for _ in 0..axis_count {
                            let idx = avp.u16()?;
                            let val = avp.fixed()? as f32 / 65536.0;
                            rec.axis_values.push((idx, val));
                        }
                    }
                    _ => {}
                }
                axis_values.push(rec);
            }
        }

        Ok(Stat {
            major_version,
            minor_version,
            design_axes,
            axis_values,
            elided_fallback_name_id,
        })
    }
}

impl Table for Stat {
    fn tag() -> Tag {
        Tag::new(b"STAT")
    }

    fn parse(_buf: &[u8], _offset: usize) -> Result<Self, FontError> {
        Err(FontError::invalid_table(
            Self::tag(),
            "STAT should be parsed via Stat::parse",
        ))
    }

    fn write(&self, w: &mut Writer) -> Result<(), FontError> {
        let axis_count = self.design_axes.len() as u16;
        let value_count = self.axis_values.len() as u16;
        let header_size = if self.elided_fallback_name_id.is_some() { 20 } else { 18 };
        let design_axis_offset = header_size;
        let axis_value_offset = design_axis_offset + axis_count as usize * 8;

        w.write_u16(self.major_version);
        w.write_u16(self.minor_version);
        w.write_u16(8); // designAxisRecordSize
        w.write_u16(axis_count);
        w.write_u32(design_axis_offset as u32);
        w.write_u16(value_count);
        w.write_u32(axis_value_offset as u32);
        if let Some(id) = self.elided_fallback_name_id {
            w.write_u16(id);
        }

        for axis in &self.design_axes {
            w.write_tag(&axis.axis_tag.0);
            w.write_u16(axis.axis_name_id);
            w.write_u16(axis.axis_ordering);
        }

        // Build axis value data first to compute sizes
        let mut value_data = Vec::new();
        let mut value_data_offsets = Vec::with_capacity(value_count as usize);
        for rec in &self.axis_values {
            value_data_offsets.push(value_data.len());
            let mut vw = Writer::new();
            vw.write_u16(rec.format);
            vw.write_u16(rec.axis_index);
            vw.write_u16(rec.flags);
            vw.write_u16(rec.value_name_id);
            match rec.format {
                1 => {
                    vw.write_fixed((rec.value * 65536.0) as i32);
                }
                2 => {
                    vw.write_fixed((rec.nominal_value.unwrap_or(0.0) * 65536.0) as i32);
                    vw.write_fixed((rec.range_min_value.unwrap_or(0.0) * 65536.0) as i32);
                    vw.write_fixed((rec.range_max_value.unwrap_or(0.0) * 65536.0) as i32);
                }
                3 => {
                    vw.write_fixed((rec.value * 65536.0) as i32);
                    vw.write_fixed((rec.linked_value.unwrap_or(0.0) * 65536.0) as i32);
                }
                4 => {
                    vw.write_u16(rec.axis_values.len() as u16);
                    for &(idx, val) in &rec.axis_values {
                        vw.write_u16(idx);
                        vw.write_fixed((val * 65536.0) as i32);
                    }
                }
                _ => {}
            }
            value_data.extend_from_slice(vw.bytes());
        }

        // Axis value array wrapper
        w.write_u16(1); // format
        w.write_u16(value_count);
        let wrapper_size = 4 + value_count as usize * 2;
        for off in value_data_offsets {
            w.write_u16((axis_value_offset + wrapper_size + off) as u16);
        }
        w.write_bytes(&value_data);

        Ok(())
    }
}
