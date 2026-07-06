use crate::error::{FontError, Tag};

pub struct Parser<'a> {
    buf: &'a [u8],
    offset: usize,
}

impl<'a> Parser<'a> {
    pub fn new(buf: &'a [u8], offset: usize) -> Self {
        Parser { buf, offset }
    }

    pub fn buf(&self) -> &'a [u8] {
        self.buf
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.offset)
    }

    pub fn seek(&mut self, offset: usize) {
        self.offset = offset;
    }

    pub fn advance(&mut self, n: usize) {
        self.offset += n;
    }

    pub fn slice(&self, len: usize) -> Result<&'a [u8], FontError> {
        let end = self.offset.checked_add(len).ok_or({
            FontError::OutOfBounds {
                offset: self.offset,
                length: len,
                buf_len: self.buf.len(),
            }
        })?;
        if end > self.buf.len() {
            return Err(FontError::OutOfBounds {
                offset: self.offset,
                length: len,
                buf_len: self.buf.len(),
            });
        }
        Ok(&self.buf[self.offset..end])
    }

    pub fn u8(&mut self) -> Result<u8, FontError> {
        let b = self.slice(1)?;
        self.offset += 1;
        Ok(b[0])
    }

    pub fn u16(&mut self) -> Result<u16, FontError> {
        let b = self.slice(2)?;
        self.offset += 2;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    pub fn i16(&mut self) -> Result<i16, FontError> {
        let b = self.slice(2)?;
        self.offset += 2;
        Ok(i16::from_be_bytes([b[0], b[1]]))
    }

    pub fn u24(&mut self) -> Result<u32, FontError> {
        let b = self.slice(3)?;
        self.offset += 3;
        Ok(u32::from_be_bytes([0, b[0], b[1], b[2]]))
    }

    pub fn u32(&mut self) -> Result<u32, FontError> {
        let b = self.slice(4)?;
        self.offset += 4;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn i32(&mut self) -> Result<i32, FontError> {
        let b = self.slice(4)?;
        self.offset += 4;
        Ok(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn i64(&mut self) -> Result<i64, FontError> {
        let b = self.slice(8)?;
        self.offset += 8;
        Ok(i64::from_be_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    pub fn u64(&mut self) -> Result<u64, FontError> {
        let b = self.slice(8)?;
        self.offset += 8;
        Ok(u64::from_be_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    pub fn tag(&mut self) -> Result<Tag, FontError> {
        let b = self.slice(4)?;
        self.offset += 4;
        Ok(Tag([b[0], b[1], b[2], b[3]]))
    }

    pub fn fixed(&mut self) -> Result<i32, FontError> {
        self.i32()
    }

    pub fn longdatetime(&mut self) -> Result<i64, FontError> {
        self.i64()
    }

    pub fn checksum(&mut self, length: usize) -> Result<u32, FontError> {
        let data = self.slice(length)?;
        Ok(checksum_table(data))
    }
}

pub fn checksum_table(data: &[u8]) -> u32 {
    let mut sum: u32 = 0;
    let mut i = 0;
    while i + 4 <= data.len() {
        sum = sum.wrapping_add(u32::from_be_bytes([
            data[i],
            data[i + 1],
            data[i + 2],
            data[i + 3],
        ]));
        i += 4;
    }
    // Handle trailing bytes (pad with zeros)
    if i < data.len() {
        let mut remainder = [0u8; 4];
        remainder[..(data.len() - i)].copy_from_slice(&data[i..]);
        sum = sum.wrapping_add(u32::from_be_bytes(remainder));
    }
    sum
}
