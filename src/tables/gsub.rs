use crate::error::FontError;
use crate::parse::Parser;
use crate::write::Writer;

#[derive(Debug, Clone, PartialEq)]
pub struct Gsub {
    pub features: Vec<String>,
    raw: Vec<u8>,
}

impl Gsub {
    pub fn tag() -> crate::error::Tag {
        crate::error::Tag::new(b"GSUB")
    }

    pub fn parse(data: &[u8]) -> Result<Self, FontError> {
        let raw = data.to_vec();
        let mut p = Parser::new(data, 0);
        let major_version = p.u16()?;
        let _minor_version = p.u16()?;
        let _script_list_offset = p.u16()? as usize;
        let feature_list_offset = p.u16()? as usize;
        let _lookup_list_offset = p.u16()? as usize;

        let mut features = Vec::new();
        if major_version >= 1 && feature_list_offset > 0 && feature_list_offset < data.len() {
            let _ = Self::parse_feature_list(data, feature_list_offset, &mut features);
        }

        Ok(Gsub { features, raw })
    }

    fn parse_feature_list(data: &[u8], offset: usize, features: &mut Vec<String>) -> Result<(), FontError> {
        let mut p = Parser::new(data, offset);
        let feature_count = p.u16()? as usize;
        for _ in 0..feature_count {
            let tag_bytes = p.slice(4)?;
            let tag = String::from_utf8_lossy(tag_bytes);
            features.push(tag.to_string());
            p.advance(4);
            let _feature_offset = p.u16()?;
        }
        Ok(())
    }

    pub fn has_ligatures(&self) -> bool {
        self.features.iter().any(|f| f == "liga" || f == "dlig" || f == "clig")
    }

    pub fn write(&self, writer: &mut Writer) -> Result<(), FontError> {
        writer.write_bytes(&self.raw);
        Ok(())
    }
}
