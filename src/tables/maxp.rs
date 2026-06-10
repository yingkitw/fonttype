use crate::error::{FontError, Tag};
use crate::parse::Parser;
use crate::tables::Table;
use crate::write::Writer;

#[derive(Debug, Clone, PartialEq)]
pub struct Maxp {
    pub version: i32, // Fixed
    pub num_glyphs: u16,
    // v1.0 fields
    pub max_points: Option<u16>,
    pub max_contours: Option<u16>,
    pub max_composite_points: Option<u16>,
    pub max_composite_contours: Option<u16>,
    pub max_zones: Option<u16>,
    pub max_twilight_points: Option<u16>,
    pub max_storage: Option<u16>,
    pub max_function_defs: Option<u16>,
    pub max_instruction_defs: Option<u16>,
    pub max_stack_elements: Option<u16>,
    pub max_size_of_instructions: Option<u16>,
    pub max_component_elements: Option<u16>,
    pub max_component_depth: Option<u16>,
}

impl Table for Maxp {
    fn tag() -> Tag {
        Tag::new(b"maxp")
    }

    fn parse(buf: &[u8], offset: usize) -> Result<Self, FontError> {
        let mut p = Parser::new(buf, offset);
        let version = p.fixed()?;
        let num_glyphs = p.u16()?;
        let mut maxp = Maxp {
            version,
            num_glyphs,
            max_points: None,
            max_contours: None,
            max_composite_points: None,
            max_composite_contours: None,
            max_zones: None,
            max_twilight_points: None,
            max_storage: None,
            max_function_defs: None,
            max_instruction_defs: None,
            max_stack_elements: None,
            max_size_of_instructions: None,
            max_component_elements: None,
            max_component_depth: None,
        };
        if version == 0x00010000 {
            maxp.max_points = Some(p.u16()?);
            maxp.max_contours = Some(p.u16()?);
            maxp.max_composite_points = Some(p.u16()?);
            maxp.max_composite_contours = Some(p.u16()?);
            maxp.max_zones = Some(p.u16()?);
            maxp.max_twilight_points = Some(p.u16()?);
            maxp.max_storage = Some(p.u16()?);
            maxp.max_function_defs = Some(p.u16()?);
            maxp.max_instruction_defs = Some(p.u16()?);
            maxp.max_stack_elements = Some(p.u16()?);
            maxp.max_size_of_instructions = Some(p.u16()?);
            maxp.max_component_elements = Some(p.u16()?);
            maxp.max_component_depth = Some(p.u16()?);
        }
        Ok(maxp)
    }

    fn write(&self, w: &mut Writer) -> Result<(), FontError> {
        w.write_fixed(self.version);
        w.write_u16(self.num_glyphs);
        if self.version == 0x00010000 {
            w.write_u16(self.max_points.unwrap_or(0));
            w.write_u16(self.max_contours.unwrap_or(0));
            w.write_u16(self.max_composite_points.unwrap_or(0));
            w.write_u16(self.max_composite_contours.unwrap_or(0));
            w.write_u16(self.max_zones.unwrap_or(0));
            w.write_u16(self.max_twilight_points.unwrap_or(0));
            w.write_u16(self.max_storage.unwrap_or(0));
            w.write_u16(self.max_function_defs.unwrap_or(0));
            w.write_u16(self.max_instruction_defs.unwrap_or(0));
            w.write_u16(self.max_stack_elements.unwrap_or(0));
            w.write_u16(self.max_size_of_instructions.unwrap_or(0));
            w.write_u16(self.max_component_elements.unwrap_or(0));
            w.write_u16(self.max_component_depth.unwrap_or(0));
        }
        Ok(())
    }
}
