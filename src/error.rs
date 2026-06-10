use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tag(pub [u8; 4]);

impl Tag {
    pub const fn new(s: &[u8; 4]) -> Self {
        Tag(*s)
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = String::from_utf8_lossy(&self.0);
        write!(f, "{}", s)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FontError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid table {tag}: {reason}")]
    InvalidTable { tag: Tag, reason: String },
    #[error("Missing table: {0}")]
    MissingTable(Tag),
    #[error("Unsupported cmap format: {0}")]
    UnsupportedCmapFormat(u16),
    #[error("Checksum mismatch for table {0}")]
    ChecksumMismatch(Tag),
    #[error("Out of bounds access at offset {offset}, length {length} in buffer of size {buf_len}")]
    OutOfBounds {
        offset: usize,
        length: usize,
        buf_len: usize,
    },
}

impl FontError {
    pub fn invalid_table(tag: Tag, reason: impl Into<String>) -> Self {
        FontError::InvalidTable {
            tag,
            reason: reason.into(),
        }
    }
}
