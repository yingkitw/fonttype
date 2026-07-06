//! Character encoding utilities for legacy single-byte encodings.
//!
//! Provides lookup tables and conversion helpers for Unicode, ASCII,
//! Latin-1 (ISO-8859-1), Windows-1252, and Mac Roman. These are useful
//! when interpreting older `cmap` subtables or font name records that
//! use platform-specific encodings rather than Unicode.

use std::collections::HashMap;

/// Supported character encoding kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodingType {
    /// Unicode (UTF-8/UTF-16).
    Unicode,
    /// 7-bit ASCII.
    Ascii,
    /// Latin-1 (ISO-8859-1).
    Latin1,
    /// Windows-1252 (Western European).
    Windows1252,
    /// Mac Roman (classic Mac OS).
    MacRoman,
    /// User-defined encoding.
    Custom,
}

/// A single character mapping entry.
#[derive(Debug, Clone)]
pub struct CharMapping {
    /// Unicode codepoint.
    pub unicode: u32,
    /// PostScript-style glyph name (e.g. "A", "space", "uni0041").
    pub glyph_name: String,
    /// Character code in this encoding.
    pub char_code: u32,
}

/// A named table of character mappings for one encoding.
#[derive(Debug, Clone)]
pub struct EncodingTable {
    pub encoding_type: EncodingType,
    pub name: String,
    pub mappings: HashMap<u32, CharMapping>,
    /// Reverse lookup from Unicode codepoint to char code.
    pub unicode_to_char: HashMap<u32, u32>,
}

/// Registry of encoding tables with convenience converters.
pub struct EncodingManager {
    tables: HashMap<String, EncodingTable>,
    default_encoding: String,
}

impl EncodingManager {
    /// Create a manager preloaded with the standard encodings.
    pub fn new() -> Self {
        let mut manager = Self {
            tables: HashMap::new(),
            default_encoding: "unicode".to_string(),
        };
        manager.add_encoding_table(Self::create_unicode_table());
        manager.add_encoding_table(Self::create_ascii_table());
        manager.add_encoding_table(Self::create_latin1_table());
        manager.add_encoding_table(Self::create_windows1252_table());
        manager.add_encoding_table(Self::create_macroman_table());
        manager
    }

    fn build_table(
        encoding_type: EncodingType,
        name: &str,
        code_to_unicode: impl Fn(u8) -> u32,
    ) -> EncodingTable {
        let mut mappings = HashMap::new();
        let mut unicode_to_char = HashMap::new();
        for code in 0u16..=255 {
            let unicode = code_to_unicode(code as u8);
            if matches!(encoding_type, EncodingType::Ascii) && unicode >= 128 {
                continue;
            }
            let glyph_name = Self::unicode_to_glyph_name(unicode);
            mappings.insert(
                code as u32,
                CharMapping {
                    unicode,
                    glyph_name,
                    char_code: code as u32,
                },
            );
            unicode_to_char.insert(unicode, code as u32);
        }
        EncodingTable {
            encoding_type,
            name: name.to_string(),
            mappings,
            unicode_to_char,
        }
    }

    fn create_unicode_table() -> EncodingTable {
        // Basic Latin + Latin-1 Supplement (U+0000..U+00FF), matching the
        // single-byte range used by the legacy encodings below.
        Self::build_table(EncodingType::Unicode, "unicode", |c| c as u32)
    }

    fn create_ascii_table() -> EncodingTable {
        Self::build_table(EncodingType::Ascii, "ascii", |c| c as u32)
    }

    fn create_latin1_table() -> EncodingTable {
        Self::build_table(EncodingType::Latin1, "latin1", |c| c as u32)
    }

    fn create_windows1252_table() -> EncodingTable {
        Self::build_table(EncodingType::Windows1252, "windows1252", Self::windows1252_to_unicode)
    }

    fn create_macroman_table() -> EncodingTable {
        Self::build_table(EncodingType::MacRoman, "macroman", Self::macroman_to_unicode)
    }

    /// Convert a Windows-1252 byte to its Unicode codepoint.
    pub fn windows1252_to_unicode(code: u8) -> u32 {
        match code {
            0x80 => 0x20AC,
            0x82 => 0x201A,
            0x83 => 0x0192,
            0x84 => 0x201E,
            0x85 => 0x2026,
            0x86 => 0x2020,
            0x87 => 0x2021,
            0x88 => 0x02C6,
            0x89 => 0x2030,
            0x8A => 0x0160,
            0x8B => 0x2039,
            0x8C => 0x0152,
            0x8E => 0x017D,
            0x91 => 0x2018,
            0x92 => 0x2019,
            0x93 => 0x201C,
            0x94 => 0x201D,
            0x95 => 0x2022,
            0x96 => 0x2013,
            0x97 => 0x2014,
            0x98 => 0x02DC,
            0x99 => 0x2122,
            0x9A => 0x0161,
            0x9B => 0x203A,
            0x9C => 0x0153,
            0x9E => 0x017E,
            0x9F => 0x0178,
            _ => code as u32,
        }
    }

    /// Convert a Mac Roman byte to its Unicode codepoint.
    pub fn macroman_to_unicode(code: u8) -> u32 {
        match code {
            0x80 => 0x00C4,
            0x81 => 0x00C5,
            0x82 => 0x00C7,
            0x83 => 0x00C9,
            0x84 => 0x00D1,
            0x85 => 0x00D6,
            0x86 => 0x00DC,
            0x87 => 0x00E1,
            0x88 => 0x00E0,
            0x89 => 0x00E2,
            0x8A => 0x00E4,
            0x8B => 0x00E3,
            0x8C => 0x00E5,
            0x8D => 0x00E7,
            0x8E => 0x00E9,
            0x8F => 0x00E8,
            _ => code as u32,
        }
    }

    /// Best-effort glyph name for a Unicode codepoint, following the common
    /// PostScript convention used in `post` and `CFF` tables.
    pub fn unicode_to_glyph_name(unicode: u32) -> String {
        match unicode {
            0x0020 => "space".to_string(),
            0x0021 => "exclam".to_string(),
            0x0022 => "quotedbl".to_string(),
            0x0023 => "numbersign".to_string(),
            0x0024 => "dollar".to_string(),
            0x0025 => "percent".to_string(),
            0x0026 => "ampersand".to_string(),
            0x0027 => "quotesingle".to_string(),
            0x0028 => "parenleft".to_string(),
            0x0029 => "parenright".to_string(),
            0x002A => "asterisk".to_string(),
            0x002B => "plus".to_string(),
            0x002C => "comma".to_string(),
            0x002D => "hyphen".to_string(),
            0x002E => "period".to_string(),
            0x002F => "slash".to_string(),
            0x0030..=0x0039 => format!("digit{}", unicode - 0x0030),
            0x0041..=0x005A | 0x0061..=0x007A => {
                char::from_u32(unicode).map(|c| c.to_string()).unwrap_or_default()
            }
            _ => format!("uni{:04X}", unicode),
        }
    }

    /// Register an additional encoding table.
    pub fn add_encoding_table(&mut self, table: EncodingTable) {
        self.tables.insert(table.name.clone(), table);
    }

    /// Look up an encoding table by name.
    pub fn get_encoding_table(&self, name: &str) -> Option<&EncodingTable> {
        self.tables.get(name)
    }

    /// Look up the default encoding table.
    pub fn get_default_encoding(&self) -> Option<&EncodingTable> {
        self.tables.get(&self.default_encoding)
    }

    /// Change the default encoding (no-op if `name` is not registered).
    pub fn set_default_encoding(&mut self, name: String) {
        if self.tables.contains_key(&name) {
            self.default_encoding = name;
        }
    }

    /// Convert a character code to Unicode using the named encoding.
    pub fn char_to_unicode(&self, char_code: u32, encoding_name: &str) -> Option<u32> {
        let table = self.tables.get(encoding_name)?;
        table.mappings.get(&char_code).map(|m| m.unicode)
    }

    /// Convert a Unicode codepoint to a character code using the named encoding.
    pub fn unicode_to_char(&self, unicode: u32, encoding_name: &str) -> Option<u32> {
        let table = self.tables.get(encoding_name)?;
        table.unicode_to_char.get(&unicode).copied()
    }

    /// Get the glyph name for a Unicode codepoint via the named encoding.
    pub fn get_glyph_name(&self, unicode: u32, encoding_name: &str) -> Option<String> {
        let table = self.tables.get(encoding_name)?;
        let char_code = table.unicode_to_char.get(&unicode)?;
        table.mappings.get(char_code).map(|m| m.glyph_name.clone())
    }

    /// List the names of all registered encodings.
    pub fn list_encodings(&self) -> Vec<String> {
        self.tables.keys().cloned().collect()
    }
}

impl Default for EncodingManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_encodings_registered() {
        let manager = EncodingManager::new();
        assert!(manager.get_encoding_table("unicode").is_some());
        assert!(manager.get_encoding_table("ascii").is_some());
        assert!(manager.get_encoding_table("latin1").is_some());
        assert!(manager.get_encoding_table("windows1252").is_some());
        assert!(manager.get_encoding_table("macroman").is_some());
    }

    #[test]
    fn test_unicode_lookup() {
        let manager = EncodingManager::new();
        assert_eq!(manager.char_to_unicode(0x41, "unicode"), Some(0x41));
    }

    #[test]
    fn test_glyph_name_lookup() {
        let manager = EncodingManager::new();
        assert_eq!(manager.get_glyph_name(0x41, "unicode").as_deref(), Some("A"));
        assert_eq!(manager.get_glyph_name(0x20, "unicode").as_deref(), Some("space"));
    }

    #[test]
    fn test_ascii_excludes_high_bytes() {
        let manager = EncodingManager::new();
        let ascii = manager.get_encoding_table("ascii").unwrap();
        assert!(ascii.mappings.len() <= 128);
        assert_eq!(manager.char_to_unicode(0x80, "ascii"), None);
    }

    #[test]
    fn test_windows1252_special_chars() {
        assert_eq!(EncodingManager::windows1252_to_unicode(0x80), 0x20AC);
        assert_eq!(EncodingManager::windows1252_to_unicode(0x99), 0x2122);
        let manager = EncodingManager::new();
        assert_eq!(manager.char_to_unicode(0x80, "windows1252"), Some(0x20AC));
    }

    #[test]
    fn test_macroman_special_chars() {
        assert_eq!(EncodingManager::macroman_to_unicode(0x80), 0x00C4);
        let manager = EncodingManager::new();
        // Forward lookup for the explicitly mapped Mac Roman bytes is unambiguous.
        assert_eq!(manager.char_to_unicode(0x80, "macroman"), Some(0x00C4));
        assert_eq!(manager.char_to_unicode(0x86, "macroman"), Some(0x00DC));
    }

    #[test]
    fn test_round_trip_latin1() {
        let manager = EncodingManager::new();
        for code in 0..=255u32 {
            let uni = manager.char_to_unicode(code, "latin1").unwrap();
            assert_eq!(manager.unicode_to_char(uni, "latin1"), Some(code));
        }
    }
}
