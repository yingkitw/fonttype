use crate::error::{FontError, Tag};
use crate::write::Writer;

pub mod head;
pub mod hhea;
pub mod maxp;
pub mod post;
pub mod name;
pub mod cmap;
pub mod os2;
pub mod glyf;
pub mod loca;
pub mod hmtx;
pub mod kern;
pub mod hinting;
pub mod gpos;
pub mod gsub;
pub mod var;
pub mod fvar;
pub mod stat;
pub mod cff;
pub mod colr;
pub mod cpal;
pub mod svg;

pub trait Table: Sized {
    fn tag() -> Tag;
    fn parse(buf: &[u8], offset: usize) -> Result<Self, FontError>;
    fn write(&self, writer: &mut Writer) -> Result<(), FontError>;
}
