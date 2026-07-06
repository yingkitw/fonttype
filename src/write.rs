pub struct Writer {
    buf: Vec<u8>,
}

impl Default for Writer {
    fn default() -> Self {
        Self::new()
    }
}

impl Writer {
    pub fn new() -> Self {
        Writer { buf: Vec::new() }
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.buf
    }

    pub fn bytes(&self) -> &[u8] {
        &self.buf
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn write_u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    pub fn write_i8(&mut self, v: i8) {
        self.buf.push(v as u8);
    }

    pub fn write_u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    pub fn write_i16(&mut self, v: i16) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    pub fn write_u24(&mut self, v: u32) {
        let bytes = v.to_be_bytes();
        self.buf.extend_from_slice(&bytes[1..4]);
    }

    pub fn write_u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    pub fn write_i32(&mut self, v: i32) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    pub fn write_u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    pub fn write_tag(&mut self, tag: &[u8; 4]) {
        self.buf.extend_from_slice(tag);
    }

    pub fn write_fixed(&mut self, v: i32) {
        self.write_i32(v);
    }

    pub fn write_longdatetime(&mut self, v: i64) {
        self.write_u64(v as u64);
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    pub fn pad_to_4(&mut self) {
        while !self.buf.len().is_multiple_of(4) {
            self.buf.push(0);
        }
    }

    pub fn checksum(&self, start: usize, length: usize) -> u32 {
        let data = &self.buf[start..start + length];
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
        sum
    }
}
