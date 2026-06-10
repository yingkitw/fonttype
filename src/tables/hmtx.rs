use crate::error::{FontError, Tag};
use crate::parse::Parser;
use crate::tables::Table;
use crate::write::Writer;

#[derive(Debug, Clone, PartialEq)]
pub struct LongHorMetricRecord {
    pub advance_width: u16,
    pub lsb: i16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hmtx {
    pub h_metrics: Vec<LongHorMetricRecord>,
    pub left_side_bearings: Vec<i16>, // for glyphs beyond hMetrics count
}

impl Hmtx {
    pub fn metric_for_glyph(&self, glyph_id: u16) -> (u16, i16) {
        if (glyph_id as usize) < self.h_metrics.len() {
            let m = &self.h_metrics[glyph_id as usize];
            (m.advance_width, m.lsb)
        } else {
            let idx = glyph_id as usize - self.h_metrics.len();
            let aw = self.h_metrics.last().map(|m| m.advance_width).unwrap_or(0);
            (aw, self.left_side_bearings.get(idx).copied().unwrap_or(0))
        }
    }
}

impl Table for Hmtx {
    fn tag() -> Tag {
        Tag::new(b"hmtx")
    }

    fn parse(_buf: &[u8], _offset: usize) -> Result<Self, FontError> {
        // Cannot parse hmtx without knowing numberOfHMetrics from hhea
        Err(FontError::invalid_table(
            Self::tag(),
            "hmtx requires numberOfHMetrics; use Hmtx::parse_with_count",
        ))
    }

    fn write(&self, w: &mut Writer) -> Result<(), FontError> {
        for m in &self.h_metrics {
            w.write_u16(m.advance_width);
            w.write_i16(m.lsb);
        }
        for &lsb in &self.left_side_bearings {
            w.write_i16(lsb);
        }
        Ok(())
    }
}

impl Hmtx {
    pub fn parse_with_count(buf: &[u8], offset: usize, num_h_metrics: u16, num_glyphs: u16) -> Result<Self, FontError> {
        let mut p = Parser::new(buf, offset);
        let mut h_metrics = Vec::with_capacity(num_h_metrics as usize);
        for _ in 0..num_h_metrics {
            h_metrics.push(LongHorMetricRecord {
                advance_width: p.u16()?,
                lsb: p.i16()?,
            });
        }
        let extra = num_glyphs.saturating_sub(num_h_metrics) as usize;
        let mut left_side_bearings = Vec::with_capacity(extra);
        for _ in 0..extra {
            left_side_bearings.push(p.i16()?);
        }
        Ok(Hmtx { h_metrics, left_side_bearings })
    }
}
