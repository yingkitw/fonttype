//! Builder-style API for modifying font metadata and metrics.
//!
//! Unlike a raw struct mutation, [`FontModifier`] offers a chainable,
//! intention-revealing API for the most common edits and keeps related
//! fields in sync (e.g. setting the family name updates the right `name`
//! records; setting ascender updates both `hhea` and `OS/2`).
//!
//! ```no_run
//! use fonttype::Font;
//!
//! let bytes = std::fs::read("font.ttf")?;
//! let mut font = Font::read(&bytes)?;
//! font.modify()
//!     .set_family("My Font")
//!     .set_version(2, 0)
//!     .set_font_metrics(2048, 1638, -410, 204);
//! std::fs::write("out.ttf", font.write()?)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::font::Font;
use crate::tables::name::NameRecord;

/// A chainable modifier borrowing a [`Font`] mutably.
pub struct FontModifier<'a> {
    font: &'a mut Font,
}

impl<'a> FontModifier<'a> {
    pub fn new(font: &'a mut Font) -> Self {
        Self { font }
    }

    fn set_name_record(&mut self, name_id: u16, value: &str) -> &mut Self {
        let platform_id = 3u16;
        let encoding_id = 1u16;
        let language_id = 0x0409u16;
        let mut found = false;
        for rec in &mut self.font.name.records {
            if rec.name_id == name_id
                && rec.platform_id == platform_id
                && rec.encoding_id == encoding_id
                && rec.language_id == language_id
            {
                rec.string = value.to_string();
                found = true;
            }
        }
        if !found {
            self.font.name.records.push(NameRecord {
                platform_id,
                encoding_id,
                language_id,
                name_id,
                string: value.to_string(),
            });
        }
        self
    }

    /// Set the font family name (name ID 1).
    pub fn set_family(&mut self, name: &str) -> &mut Self {
        self.font.name.set_family(name);
        self
    }

    /// Set the subfamily / style name (name ID 2).
    pub fn set_subfamily(&mut self, name: &str) -> &mut Self {
        self.font.name.set_subfamily(name);
        self
    }

    /// Set the full font name (name ID 4).
    pub fn set_full_name(&mut self, name: &str) -> &mut Self {
        self.set_name_record(4, name)
    }

    /// Set the version string (name ID 5) and `head.fontRevision`.
    pub fn set_version(&mut self, major: u16, minor: u16) -> &mut Self {
        self.set_name_record(5, &format!("Version {}.{}", major, minor));
        // Fixed 16.16: integer part in the high 16 bits, fraction in the low 16.
        let frac = ((minor as i32).saturating_mul(65536)) / 100;
        self.font.head.font_revision = ((major as i32) << 16) | (frac & 0xFFFF);
        self
    }

    /// Set the copyright notice (name ID 0).
    pub fn set_copyright(&mut self, text: &str) -> &mut Self {
        self.set_name_record(0, text)
    }

    /// Set the trademark notice (name ID 7).
    pub fn set_trademark(&mut self, text: &str) -> &mut Self {
        self.set_name_record(7, text)
    }

    /// Set `head.unitsPerEm`.
    pub fn set_units_per_em(&mut self, units_per_em: u16) -> &mut Self {
        self.font.head.units_per_em = units_per_em;
        self
    }

    /// Set vertical metrics in both `hhea` and `OS/2`.
    pub fn set_vertical_metrics(&mut self, ascender: i16, descender: i16, line_gap: i16) -> &mut Self {
        self.font.hhea.ascender = ascender;
        self.font.hhea.descender = descender;
        self.font.hhea.line_gap = line_gap;
        self.font.os2.s_typo_ascender = ascender;
        self.font.os2.s_typo_descender = descender;
        self.font.os2.s_typo_line_gap = line_gap;
        self
    }

    /// Convenience for setting `unitsPerEm` and vertical metrics together.
    pub fn set_font_metrics(
        &mut self,
        units_per_em: u16,
        ascender: i16,
        descender: i16,
        line_gap: i16,
    ) -> &mut Self {
        self.set_units_per_em(units_per_em)
            .set_vertical_metrics(ascender, descender, line_gap)
    }

    /// Set embedding permissions (`OS/2.fsType`).
    pub fn set_embedding_type(&mut self, fs_type: u16) -> &mut Self {
        self.font.os2.fs_type = fs_type;
        self
    }

    /// Set the advance width of a single glyph, keeping `hhea.advanceWidthMax` in sync.
    pub fn set_glyph_advance(&mut self, glyph_index: usize, advance_width: u16) -> &mut Self {
        if let Some(metric) = self.font.hmtx.h_metrics.get_mut(glyph_index) {
            metric.advance_width = advance_width;
        }
        let max = self
            .font
            .hmtx
            .h_metrics
            .iter()
            .map(|m| m.advance_width)
            .max()
            .unwrap_or(0);
        self.font.hhea.advance_width_max = max;
        self
    }
}

impl Font {
    /// Begin a chain of modifications on this font.
    pub fn modify(&mut self) -> FontModifier<'_> {
        FontModifier::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modifier_updates_family_and_version() {
        let mut font = Font::create_minimal();
        font.modify()
            .set_family("Renamed")
            .set_version(2, 1);
        assert_eq!(font.name.family_name().as_deref(), Some("Renamed"));
        // Integer part of fontRevision is 2.
        assert_eq!(font.head.font_revision >> 16, 2);
        let written = font.write().unwrap();
        let reread = Font::read(&written).unwrap();
        assert_eq!(reread.name.family_name().as_deref(), Some("Renamed"));
    }

    #[test]
    fn test_modifier_vertical_metrics_synced() {
        let mut font = Font::create_minimal();
        font.modify().set_font_metrics(2048, 1800, -400, 100);
        assert_eq!(font.head.units_per_em, 2048);
        assert_eq!(font.hhea.ascender, 1800);
        assert_eq!(font.os2.s_typo_ascender, 1800);
        assert_eq!(font.hhea.descender, -400);
    }

    #[test]
    fn test_modifier_glyph_advance_and_max() {
        let mut font = Font::create_minimal();
        font.modify().set_glyph_advance(0, 750);
        assert_eq!(font.hmtx.h_metrics[0].advance_width, 750);
        assert_eq!(font.hhea.advance_width_max, 750);
    }

    #[test]
    fn test_modifier_adds_missing_name_record() {
        let mut font = Font::create_minimal();
        font.modify().set_full_name("TestFont Regular");
        assert_eq!(font.name.full_name().as_deref(), Some("TestFont Regular"));
    }
}
