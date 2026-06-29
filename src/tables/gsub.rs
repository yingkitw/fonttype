use crate::error::FontError;
use crate::parse::Parser;
use crate::write::Writer;

#[derive(Debug, Clone, PartialEq)]
pub struct Gsub {
    pub scripts: Vec<ScriptRecord>,
    pub features: Vec<FeatureRecord>,
    pub lookups: Vec<Lookup>,
    raw: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScriptRecord {
    pub script_tag: String,
    pub default_lang_sys: Option<LangSys>,
    pub lang_sys_records: Vec<LangSysRecord>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LangSysRecord {
    pub lang_sys_tag: String,
    pub lang_sys: LangSys,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LangSys {
    pub required_feature_index: u16,
    pub lookup_indices: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeatureRecord {
    pub feature_tag: String,
    pub feature: Feature,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Feature {
    pub feature_params: u16,
    pub lookup_indices: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Lookup {
    pub lookup_type: u16,
    pub lookup_flag: u16,
    pub subtables: Vec<Subtable>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Subtable {
    SingleSubst { coverage: Coverage, delta: i16 },
    MultipleSubst { coverage: Coverage, sequences: Vec<Vec<u16>> },
    AlternateSubst { coverage: Coverage, alternate_sets: Vec<Vec<u16>> },
    LigatureSubst { coverage: Coverage, ligature_sets: Vec<LigatureSet> },
    Passthrough { coverage: Coverage },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Coverage {
    pub format: u16,
    pub glyphs: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LigatureSet {
    pub ligatures: Vec<Ligature>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ligature {
    pub lig_glyph: u16,
    pub components: Vec<u16>,
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
        let script_list_offset = p.u16()? as usize;
        let feature_list_offset = p.u16()? as usize;
        let lookup_list_offset = p.u16()? as usize;

        let scripts = if major_version >= 1 && script_list_offset > 0 && script_list_offset < data.len() {
            parse_script_list(data, script_list_offset).unwrap_or_default()
        } else {
            Vec::new()
        };

        let features = if major_version >= 1 && feature_list_offset > 0 && feature_list_offset < data.len() {
            parse_feature_list(data, feature_list_offset).unwrap_or_default()
        } else {
            Vec::new()
        };

        let lookups = if lookup_list_offset > 0 && lookup_list_offset < data.len() {
            parse_lookup_list(data, lookup_list_offset).unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(Gsub { scripts, features, lookups, raw })
    }

    pub fn has_ligatures(&self) -> bool {
        self.features.iter().any(|f| f.feature_tag == "liga" || f.feature_tag == "dlig" || f.feature_tag == "clig")
    }

    pub fn write(&self, writer: &mut Writer) -> Result<(), FontError> {
        writer.write_bytes(&self.raw);
        Ok(())
    }
}

fn parse_script_list(data: &[u8], offset: usize) -> Result<Vec<ScriptRecord>, FontError> {
    let mut p = Parser::new(data, offset);
    let count = p.u16()? as usize;
    let mut records = Vec::with_capacity(count);
    let mut offsets = Vec::with_capacity(count);
    let mut tags = Vec::with_capacity(count);
    for _ in 0..count {
        let tag_bytes = p.slice(4)?;
        tags.push(String::from_utf8_lossy(tag_bytes).to_string());
        p.advance(4);
        offsets.push(p.u16()? as usize);
    }
    for (i, off) in offsets.iter().enumerate() {
        let script_offset = offset + off;
        let mut sp = Parser::new(data, script_offset);
        let default_lang_sys_offset = sp.u16()? as usize;
        let lang_sys_count = sp.u16()? as usize;

        let default_lang_sys = if default_lang_sys_offset > 0 {
            Some(parse_lang_sys(data, script_offset + default_lang_sys_offset)?)
        } else {
            None
        };

        let mut lang_sys_records = Vec::with_capacity(lang_sys_count);
        for _ in 0..lang_sys_count {
            let tag_bytes = sp.slice(4)?;
            let lang_tag = String::from_utf8_lossy(tag_bytes).to_string();
            sp.advance(4);
            let lang_sys_off = sp.u16()? as usize;
            let lang_sys = parse_lang_sys(data, script_offset + lang_sys_off)?;
            lang_sys_records.push(LangSysRecord { lang_sys_tag: lang_tag, lang_sys });
        }

        records.push(ScriptRecord {
            script_tag: tags[i].clone(),
            default_lang_sys,
            lang_sys_records,
        });
    }
    Ok(records)
}

fn parse_lang_sys(data: &[u8], offset: usize) -> Result<LangSys, FontError> {
    let mut p = Parser::new(data, offset);
    let lookup_order = p.u16()?;
    let required_feature_index = p.u16()?;
    let feature_index_count = p.u16()? as usize;
    let mut lookup_indices = Vec::with_capacity(feature_index_count);
    for _ in 0..feature_index_count {
        lookup_indices.push(p.u16()?);
    }
    Ok(LangSys { required_feature_index, lookup_indices })
}

fn parse_feature_list(data: &[u8], offset: usize) -> Result<Vec<FeatureRecord>, FontError> {
    let mut p = Parser::new(data, offset);
    let count = p.u16()? as usize;
    let mut records = Vec::with_capacity(count);
    let mut offsets = Vec::with_capacity(count);
    let mut tags = Vec::with_capacity(count);
    for _ in 0..count {
        let tag_bytes = p.slice(4)?;
        tags.push(String::from_utf8_lossy(tag_bytes).to_string());
        p.advance(4);
        offsets.push(p.u16()? as usize);
    }
    for (i, off) in offsets.iter().enumerate() {
        let feature = parse_feature(data, offset + off)?;
        records.push(FeatureRecord { feature_tag: tags[i].clone(), feature });
    }
    Ok(records)
}

fn parse_feature(data: &[u8], offset: usize) -> Result<Feature, FontError> {
    let mut p = Parser::new(data, offset);
    let feature_params = p.u16()?;
    let lookup_index_count = p.u16()? as usize;
    let mut lookup_indices = Vec::with_capacity(lookup_index_count);
    for _ in 0..lookup_index_count {
        lookup_indices.push(p.u16()?);
    }
    Ok(Feature { feature_params, lookup_indices })
}

fn parse_lookup_list(data: &[u8], offset: usize) -> Result<Vec<Lookup>, FontError> {
    let mut p = Parser::new(data, offset);
    let count = p.u16()? as usize;
    let mut offsets = Vec::with_capacity(count);
    for _ in 0..count {
        offsets.push(p.u16()? as usize);
    }
    let mut lookups = Vec::with_capacity(count);
    for off in offsets {
        lookups.push(parse_lookup(data, offset + off)?);
    }
    Ok(lookups)
}

fn parse_lookup(data: &[u8], offset: usize) -> Result<Lookup, FontError> {
    let mut p = Parser::new(data, offset);
    let lookup_type = p.u16()?;
    let lookup_flag = p.u16()?;
    let subtable_count = p.u16()? as usize;
    let mut subtable_offsets = Vec::with_capacity(subtable_count);
    for _ in 0..subtable_count {
        subtable_offsets.push(p.u16()? as usize);
    }

    let mut subtables = Vec::with_capacity(subtable_count);
    for st_off in subtable_offsets {
        let st = parse_subtable(data, offset + st_off, lookup_type)?;
        subtables.push(st);
    }

    Ok(Lookup { lookup_type, lookup_flag, subtables })
}

fn parse_subtable(data: &[u8], offset: usize, lookup_type: u16) -> Result<Subtable, FontError> {
    let coverage = parse_coverage(data, offset)?;
    let coverage_size = coverage_size(data, offset)?;
    let mut p = Parser::new(data, offset + coverage_size);

    match lookup_type {
        1 => {
            // Single substitution
            let format = p.u16()?;
            if format == 1 {
                let delta = p.i16()?;
                Ok(Subtable::SingleSubst { coverage, delta })
            } else {
                Ok(Subtable::Passthrough { coverage })
            }
        }
        2 => {
            // Multiple substitution
            let _format = p.u16()?;
            let seq_count = p.u16()? as usize;
            let mut seq_offsets = Vec::with_capacity(seq_count);
            for _ in 0..seq_count {
                seq_offsets.push(p.u16()? as usize);
            }
            let mut sequences = Vec::with_capacity(seq_count);
            for off in seq_offsets {
                let mut sp = Parser::new(data, offset + coverage_size + off);
                let glyph_count = sp.u16()? as usize;
                let mut seq = Vec::with_capacity(glyph_count);
                for _ in 0..glyph_count {
                    seq.push(sp.u16()?);
                }
                sequences.push(seq);
            }
            Ok(Subtable::MultipleSubst { coverage, sequences })
        }
        3 => {
            // Alternate substitution
            let _format = p.u16()?;
            let alt_set_count = p.u16()? as usize;
            let mut alt_offsets = Vec::with_capacity(alt_set_count);
            for _ in 0..alt_set_count {
                alt_offsets.push(p.u16()? as usize);
            }
            let mut alternate_sets = Vec::with_capacity(alt_set_count);
            for off in alt_offsets {
                let mut ap = Parser::new(data, offset + coverage_size + off);
                let glyph_count = ap.u16()? as usize;
                let mut set = Vec::with_capacity(glyph_count);
                for _ in 0..glyph_count {
                    set.push(ap.u16()?);
                }
                alternate_sets.push(set);
            }
            Ok(Subtable::AlternateSubst { coverage, alternate_sets })
        }
        4 => {
            // Ligature substitution
            let _format = p.u16()?;
            let lig_set_count = p.u16()? as usize;
            let mut lig_set_offsets = Vec::with_capacity(lig_set_count);
            for _ in 0..lig_set_count {
                lig_set_offsets.push(p.u16()? as usize);
            }
            let mut ligature_sets = Vec::with_capacity(lig_set_count);
            for off in lig_set_offsets {
                let mut lp = Parser::new(data, offset + coverage_size + off);
                let lig_count = lp.u16()? as usize;
                let mut lig_offsets = Vec::with_capacity(lig_count);
                for _ in 0..lig_count {
                    lig_offsets.push(lp.u16()? as usize);
                }
                let mut ligatures = Vec::with_capacity(lig_count);
                for lig_off in lig_offsets {
                    let mut lgp = Parser::new(data, offset + coverage_size + off + lig_off);
                    let lig_glyph = lgp.u16()?;
                    let comp_count = lgp.u16()? as usize;
                    let mut components = Vec::with_capacity(comp_count - 1);
                    for _ in 1..comp_count {
                        components.push(lgp.u16()?);
                    }
                    ligatures.push(Ligature { lig_glyph, components });
                }
                ligature_sets.push(LigatureSet { ligatures });
            }
            Ok(Subtable::LigatureSubst { coverage, ligature_sets })
        }
        _ => Ok(Subtable::Passthrough { coverage }),
    }
}

fn parse_coverage(data: &[u8], offset: usize) -> Result<Coverage, FontError> {
    let mut p = Parser::new(data, offset);
    let format = p.u16()?;
    let glyphs = if format == 1 {
        let count = p.u16()? as usize;
        let mut g = Vec::with_capacity(count);
        for _ in 0..count {
            g.push(p.u16()?);
        }
        g
    } else if format == 2 {
        let count = p.u16()? as usize;
        let mut g = Vec::new();
        for _ in 0..count {
            let start = p.u16()?;
            let end = p.u16()?;
            let _ = p.u16()?;
            for gid in start..=end {
                g.push(gid);
            }
        }
        g
    } else {
        Vec::new()
    };
    Ok(Coverage { format, glyphs })
}

fn coverage_size(data: &[u8], offset: usize) -> Result<usize, FontError> {
    let mut p = Parser::new(data, offset);
    let format = p.u16()?;
    if format == 1 {
        let count = p.u16()? as usize;
        Ok(4 + count * 2)
    } else if format == 2 {
        let count = p.u16()? as usize;
        Ok(4 + count * 6)
    } else {
        Ok(4)
    }
}
