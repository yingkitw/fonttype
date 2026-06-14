use crate::error::{FontError, Tag};
use crate::parse::Parser;
use crate::tables::Table;
use crate::write::Writer;

#[derive(Debug, Clone, PartialEq)]
pub struct Fvar {
    pub major_version: u16,
    pub minor_version: u16,
    pub axes: Vec<AxisRecord>,
    pub instances: Vec<InstanceRecord>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AxisRecord {
    pub axis_tag: Tag,
    pub min_value: f32,
    pub default_value: f32,
    pub max_value: f32,
    pub flags: u16,
    pub axis_name_id: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstanceRecord {
    pub subfamily_name_id: u16,
    pub flags: u16,
    pub coordinates: Vec<f32>,
    pub post_script_name_id: Option<u16>,
}

impl Fvar {
    pub fn parse(data: &[u8]) -> Result<Self, FontError> {
        let mut p = Parser::new(data, 0);
        let major_version = p.u16()?;
        let minor_version = p.u16()?;
        let axes_array_offset = p.u16()?;
        let _reserved = p.u16()?;
        let axis_count = p.u16()?;
        let axis_size = p.u16()?;
        let instance_count = p.u16()?;
        let instance_size = p.u16()?;

        let mut axes = Vec::with_capacity(axis_count as usize);
        let axes_offset = axes_array_offset as usize;
        for i in 0..axis_count {
            let off = axes_offset + i as usize * axis_size as usize;
            let mut ap = Parser::new(data, off);
            let tag_bytes = ap.tag()?;
            let min_value = ap.fixed()? as f32 / 65536.0;
            let default_value = ap.fixed()? as f32 / 65536.0;
            let max_value = ap.fixed()? as f32 / 65536.0;
            let flags = ap.u16()?;
            let axis_name_id = ap.u16()?;
            axes.push(AxisRecord {
                axis_tag: tag_bytes,
                min_value,
                default_value,
                max_value,
                flags,
                axis_name_id,
            });
        }

        let has_ps_name_id = instance_size as usize >= axis_count as usize * 4 + 6;
        let mut instances = Vec::with_capacity(instance_count as usize);
        let instances_offset = axes_offset + axis_count as usize * axis_size as usize;
        for i in 0..instance_count {
            let off = instances_offset + i as usize * instance_size as usize;
            let mut ip = Parser::new(data, off);
            let subfamily_name_id = ip.u16()?;
            let flags = ip.u16()?;
            let mut coordinates = Vec::with_capacity(axis_count as usize);
            for _ in 0..axis_count {
                coordinates.push(ip.fixed()? as f32 / 65536.0);
            }
            let post_script_name_id = if has_ps_name_id {
                Some(ip.u16()?)
            } else {
                None
            };
            instances.push(InstanceRecord {
                subfamily_name_id,
                flags,
                coordinates,
                post_script_name_id,
            });
        }

        Ok(Fvar {
            major_version,
            minor_version,
            axes,
            instances,
        })
    }
}

impl Table for Fvar {
    fn tag() -> Tag {
        Tag::new(b"fvar")
    }

    fn parse(_buf: &[u8], _offset: usize) -> Result<Self, FontError> {
        Err(FontError::invalid_table(
            Self::tag(),
            "fvar should be parsed via Fvar::parse",
        ))
    }

    fn write(&self, w: &mut Writer) -> Result<(), FontError> {
        w.write_u16(self.major_version);
        w.write_u16(self.minor_version);
        let axis_count = self.axes.len() as u16;
        let instance_count = self.instances.len() as u16;
        let axis_size = 20u16;
        let has_ps = self.instances.iter().any(|i| i.post_script_name_id.is_some());
        let instance_size = if has_ps {
            axis_count * 4 + 6
        } else {
            axis_count * 4 + 4
        };
        let header_size = 16usize;
        w.write_u16(header_size as u16); // axesArrayOffset
        w.write_u16(0); // reserved
        w.write_u16(axis_count);
        w.write_u16(axis_size);
        w.write_u16(instance_count);
        w.write_u16(instance_size);

        for axis in &self.axes {
            w.write_tag(&axis.axis_tag.0);
            w.write_fixed((axis.min_value * 65536.0) as i32);
            w.write_fixed((axis.default_value * 65536.0) as i32);
            w.write_fixed((axis.max_value * 65536.0) as i32);
            w.write_u16(axis.flags);
            w.write_u16(axis.axis_name_id);
        }

        for inst in &self.instances {
            w.write_u16(inst.subfamily_name_id);
            w.write_u16(inst.flags);
            for &coord in &inst.coordinates {
                w.write_fixed((coord * 65536.0) as i32);
            }
            if let Some(ps) = inst.post_script_name_id {
                w.write_u16(ps);
            }
        }

        Ok(())
    }
}
