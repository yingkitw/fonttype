use crate::error::{FontError, Tag};
use crate::parse::Parser;
use crate::tables::Table;
use crate::write::Writer;

#[derive(Debug, Clone, PartialEq)]
pub struct KernSubtable {
    pub version: u16,
    pub length: u16,
    pub coverage: u16,
    pub pairs: Vec<KernPair>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KernPair {
    pub left: u16,
    pub right: u16,
    pub value: i16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Kern {
    pub version: u16,
    pub n_tables: u16,
    pub subtables: Vec<KernSubtable>,
}

impl Kern {
    pub fn lookup(&self, left: u16, right: u16) -> Option<i16> {
        for sub in &self.subtables {
            for pair in &sub.pairs {
                if pair.left == left && pair.right == right {
                    return Some(pair.value);
                }
            }
        }
        None
    }
}

impl Table for Kern {
    fn tag() -> Tag {
        Tag::new(b"kern")
    }

    fn parse(buf: &[u8], offset: usize) -> Result<Self, FontError> {
        let mut p = Parser::new(buf, offset);
        let version = p.u16()?;
        let n_tables = p.u16()?;
        let mut subtables = Vec::with_capacity(n_tables as usize);
        for _ in 0..n_tables {
            let sub_version = p.u16()?;
            let length = p.u16()?;
            let coverage = p.u16()?;
            let n_pairs = p.u16()?;
            let _search_range = p.u16()?;
            let _entry_selector = p.u16()?;
            let _range_shift = p.u16()?;
            let mut pairs = Vec::with_capacity(n_pairs as usize);
            for _ in 0..n_pairs {
                pairs.push(KernPair {
                    left: p.u16()?,
                    right: p.u16()?,
                    value: p.i16()?,
                });
            }
            subtables.push(KernSubtable {
                version: sub_version,
                length,
                coverage,
                pairs,
            });
        }
        Ok(Kern { version, n_tables, subtables })
    }

    fn write(&self, w: &mut Writer) -> Result<(), FontError> {
        w.write_u16(self.version);
        w.write_u16(self.subtables.len() as u16);
        for sub in &self.subtables {
            w.write_u16(sub.version);
            let length = 14 + sub.pairs.len() * 6;
            w.write_u16(length as u16);
            w.write_u16(sub.coverage);
            w.write_u16(sub.pairs.len() as u16);
            let search_range = 1u16 << (sub.pairs.len() as u32).ilog2();
            w.write_u16(search_range * 6);
            let entry_selector = (sub.pairs.len() as u32).ilog2() as u16;
            w.write_u16(entry_selector);
            let range_shift = sub.pairs.len() as u16 * 6 - search_range * 6;
            w.write_u16(range_shift);
            for pair in &sub.pairs {
                w.write_u16(pair.left);
                w.write_u16(pair.right);
                w.write_i16(pair.value);
            }
        }
        Ok(())
    }
}
