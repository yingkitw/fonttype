//! Structured font validation.
//!
//! [`Font::validate_report`] runs the same structural and checksum checks
//! as [`Font::validate`](crate::Font::validate) but returns a typed
//! [`ValidationReport`] so callers can branch on issue category and
//! severity programmatically instead of matching on strings.

use crate::error::Tag;
use crate::font::Font;
use crate::parse::checksum_table;
use crate::tables::Table;
use crate::tables::{cmap::Cmap, head::Head, hhea::Hhea, hmtx::Hmtx, maxp::Maxp, name::Name, os2::Os2};

/// Whether a validation issue blocks font validity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueSeverity {
    /// A structural problem that makes the font invalid.
    Error,
    /// A cosmetic or soft issue that does not block validity.
    Warning,
}

/// Coarse classification of a validation issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueCategory {
    /// A required table is missing.
    MissingRequiredTable,
    /// `maxp.numGlyphs` / `loca` / `glyf` glyph counts disagree.
    GlyphCount,
    /// `hmtx` entry count disagrees with glyph count.
    HmtxCount,
    /// Two tables occupy overlapping byte ranges.
    TableOverlap,
    /// Table directory is not sorted by tag.
    DirectorySort,
    /// `head.magicNumber` is wrong.
    HeadMagic,
    /// `head.unitsPerEm` is outside the valid range.
    HeadUnitsPerEm,
    /// `hhea.numberOfHMetrics` exceeds glyph count.
    HheaMetrics,
    /// `glyf` and `loca` are not both present or both absent.
    GlyfLoca,
    /// A table extends beyond the end of the file.
    TableBounds,
    /// A recorded checksum does not match the recomputed value.
    Checksum,
}

/// A single validation finding.
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: IssueSeverity,
    pub category: IssueCategory,
    pub message: String,
    pub table: Option<Tag>,
}

impl ValidationIssue {
    fn new(severity: IssueSeverity, category: IssueCategory, message: impl Into<String>) -> Self {
        Self {
            severity,
            category,
            message: message.into(),
            table: None,
        }
    }

    fn with_table(mut self, tag: Tag) -> Self {
        self.table = Some(tag);
        self
    }
}

/// A typed validation report.
#[derive(Debug, Clone, Default)]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    /// True when there are no error-severity issues.
    pub fn is_valid(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|i| i.severity == IssueSeverity::Error)
    }

    /// Iterate over error-severity issues.
    pub fn errors(&self) -> impl Iterator<Item = &ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Error)
    }

    /// Iterate over warning-severity issues.
    pub fn warnings(&self) -> impl Iterator<Item = &ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Warning)
    }

    /// Human-readable summary, mirroring the CLI output.
    pub fn summary(&self) -> String {
        if self.issues.is_empty() {
            return "valid".to_string();
        }
        let mut out = format!("{} issue(s)\n", self.issues.len());
        for issue in &self.issues {
            let sev = match issue.severity {
                IssueSeverity::Error => "error",
                IssueSeverity::Warning => "warning",
            };
            let tag = issue
                .table
                .map(|t| format!("[{}] ", t))
                .unwrap_or_default();
            out.push_str(&format!("  - {} {}{}\n", sev, tag, issue.message));
        }
        out
    }
}

impl Font {
    /// Run structural validation and return a typed report.
    pub fn validate_report(&self, buf: &[u8]) -> ValidationReport {
        let mut issues = Vec::new();

        // Required tables
        let required = [
            Head::tag(),
            Hhea::tag(),
            Maxp::tag(),
            Name::tag(),
            Cmap::tag(),
            Os2::tag(),
            Hmtx::tag(),
        ];
        for tag in &required {
            if !self.tables.iter().any(|t| t.tag == *tag) {
                issues.push(
                    ValidationIssue::new(
                        IssueSeverity::Error,
                        IssueCategory::MissingRequiredTable,
                        format!("Missing required table: {}", tag),
                    )
                    .with_table(*tag),
                );
            }
        }

        // numGlyphs consistency
        let glyf_count = self.glyf.as_ref().map(|g| g.glyphs.len()).unwrap_or(0);
        if self.maxp.num_glyphs as usize != glyf_count {
            issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                IssueCategory::GlyphCount,
                format!(
                    "maxp.numGlyphs ({}) != glyf glyph count ({})",
                    self.maxp.num_glyphs, glyf_count
                ),
            ));
        }
        if let Some(ref loca) = self.loca
            && loca.offsets.len().saturating_sub(1) != glyf_count {
                issues.push(ValidationIssue::new(
                    IssueSeverity::Error,
                    IssueCategory::GlyphCount,
                    format!(
                        "loca entry count ({}) != glyf glyph count ({})",
                        loca.offsets.len().saturating_sub(1),
                        glyf_count
                    ),
                ));
            }
        let hmtx_count = self.hmtx.h_metrics.len() + self.hmtx.left_side_bearings.len();
        if hmtx_count != glyf_count {
            issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                IssueCategory::HmtxCount,
                format!(
                    "hmtx entry count ({}) != glyph count ({})",
                    hmtx_count, glyf_count
                ),
            ));
        }

        // Table overlap detection
        for i in 0..self.tables.len() {
            for j in (i + 1)..self.tables.len() {
                let a = &self.tables[i];
                let b = &self.tables[j];
                let a_start = a.offset;
                let a_end = a.offset + a.length;
                let b_start = b.offset;
                let b_end = b.offset + b.length;
                if a_start < b_end && b_start < a_end {
                    issues.push(ValidationIssue::new(
                        IssueSeverity::Error,
                        IssueCategory::TableOverlap,
                        format!(
                            "Tables {} and {} overlap ({}..{} vs {}..{})",
                            a.tag, b.tag, a_start, a_end, b_start, b_end
                        ),
                    ));
                }
            }
        }

        // Table directory sorting
        for i in 1..self.tables.len() {
            if self.tables[i - 1].tag.0 > self.tables[i].tag.0 {
                issues.push(ValidationIssue::new(
                    IssueSeverity::Warning,
                    IssueCategory::DirectorySort,
                    format!(
                        "Table directory not sorted: {} before {}",
                        self.tables[i - 1].tag, self.tables[i].tag
                    ),
                ));
            }
        }

        // head structural checks
        if self.head.magic_number != 0x5F0F3CF5 {
            issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                IssueCategory::HeadMagic,
                format!(
                    "head.magicNumber is 0x{:08X}, expected 0x5F0F3CF5",
                    self.head.magic_number
                ),
            ));
        }
        if self.head.units_per_em < 16 || self.head.units_per_em > 16384 {
            issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                IssueCategory::HeadUnitsPerEm,
                format!(
                    "head.unitsPerEm is {}, expected 16..16384",
                    self.head.units_per_em
                ),
            ));
        }

        // hhea consistency
        if self.hhea.number_of_hmetrics > self.maxp.num_glyphs {
            issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                IssueCategory::HheaMetrics,
                format!(
                    "hhea.numberOfHMetrics ({}) > maxp.numGlyphs ({})",
                    self.hhea.number_of_hmetrics, self.maxp.num_glyphs
                ),
            ));
        }

        // glyf and loca co-presence
        let has_glyf = self.tables.iter().any(|t| t.tag == Tag::new(b"glyf"));
        let has_loca = self.tables.iter().any(|t| t.tag == Tag::new(b"loca"));
        if has_glyf != has_loca {
            issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                IssueCategory::GlyfLoca,
                format!(
                    "glyf and loca must both be present or both absent (glyf={}, loca={})",
                    has_glyf, has_loca
                ),
            ));
        }

        // Checksums from the raw buffer
        for rec in &self.tables {
            let start = rec.offset as usize;
            let end = start + rec.length as usize;
            if end > buf.len() {
                issues.push(
                    ValidationIssue::new(
                        IssueSeverity::Error,
                        IssueCategory::TableBounds,
                        format!(
                            "Table {} extends beyond file (offset {}, length {}, file {})",
                            rec.tag, rec.offset, rec.length, buf.len()
                        ),
                    )
                    .with_table(rec.tag),
                );
                continue;
            }
            let data = &buf[start..end];
            let mut padded = data.to_vec();
            while !padded.len().is_multiple_of(4) {
                padded.push(0);
            }
            if rec.tag == Head::tag() {
                let mut head_data = padded.clone();
                if head_data.len() >= 12 {
                    head_data[8..12].copy_from_slice(&[0, 0, 0, 0]);
                }
                let head_checksum = checksum_table(&head_data);
                if head_checksum != rec.checksum {
                    issues.push(
                        ValidationIssue::new(
                            IssueSeverity::Warning,
                            IssueCategory::Checksum,
                            format!(
                                "head checksum mismatch: computed {} != recorded {}",
                                head_checksum, rec.checksum
                            ),
                        )
                        .with_table(rec.tag),
                    );
                }
            } else {
                let computed = checksum_table(&padded);
                if computed != rec.checksum {
                    issues.push(
                        ValidationIssue::new(
                            IssueSeverity::Warning,
                            IssueCategory::Checksum,
                            format!(
                                "Table {} checksum mismatch: computed {} != recorded {}",
                                rec.tag, computed, rec.checksum
                            ),
                        )
                        .with_table(rec.tag),
                    );
                }
            }
        }

        ValidationReport { issues }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_is_valid_for_minimal_font() {
        let font = Font::create_minimal();
        let bytes = font.write().unwrap();
        // create_minimal() has an empty table directory, so read it back to
        // populate `tables` before validating.
        let reread = Font::read(&bytes).unwrap();
        let report = reread.validate_report(&bytes);
        assert!(report.is_valid(), "{}", report.summary());
    }

    #[test]
    fn test_report_categorizes_bad_magic() {
        let font = Font::create_minimal();
        let bytes = font.write().unwrap();
        let mut bad = Font::read(&bytes).unwrap();
        bad.head.magic_number = 0xDEADBEEF;
        let report = bad.validate_report(&bytes);
        assert!(!report.is_valid());
        assert!(report
            .errors()
            .any(|i| i.category == IssueCategory::HeadMagic));
    }

    #[test]
    fn test_report_detects_overlap() {
        let font = Font::create_minimal();
        let bytes = font.write().unwrap();
        let mut bad = Font::read(&bytes).unwrap();
        if bad.tables.len() >= 2 {
            bad.tables[1].offset = bad.tables[0].offset;
            bad.tables[1].length = bad.tables[0].length;
        }
        let report = bad.validate_report(&bytes);
        assert!(report
            .errors()
            .any(|i| i.category == IssueCategory::TableOverlap));
    }

    #[test]
    fn test_report_checksum_is_warning() {
        let font = Font::create_minimal();
        let bytes = font.write().unwrap();
        let mut bad = Font::read(&bytes).unwrap();
        if let Some(rec) = bad.tables.iter_mut().find(|t| t.tag == Tag::new(b"post")) {
            rec.checksum = rec.checksum.wrapping_add(1);
        }
        let report = bad.validate_report(&bytes);
        // A checksum mismatch is a warning, not an error, so still valid.
        assert!(report.is_valid());
        assert!(report
            .warnings()
            .any(|i| i.category == IssueCategory::Checksum));
    }
}
