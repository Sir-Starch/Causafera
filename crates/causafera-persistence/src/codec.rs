use crate::error::PersistenceError;

/// Canonical little-endian encoder with checked bounds.
#[derive(Debug)]
pub struct LittleEndianEncoder<'a> {
    buf: &'a mut Vec<u8>,
    written: usize,
}

impl<'a> LittleEndianEncoder<'a> {
    pub fn new(buf: &'a mut Vec<u8>) -> Self {
        Self { buf, written: 0 }
    }

    pub fn written(&self) -> usize {
        self.written
    }

    pub fn finish(self) -> &'a [u8] {
        self.buf
    }

    pub fn write_u8(&mut self, value: u8) {
        self.buf.push(value);
        self.written += 1;
    }

    pub fn write_u16(&mut self, value: u16) {
        self.buf.extend_from_slice(&value.to_le_bytes());
        self.written += 2;
    }

    pub fn write_u32(&mut self, value: u32) {
        self.buf.extend_from_slice(&value.to_le_bytes());
        self.written += 4;
    }

    pub fn write_u64(&mut self, value: u64) {
        self.buf.extend_from_slice(&value.to_le_bytes());
        self.written += 8;
    }

    pub fn write_i64(&mut self, value: i64) {
        self.buf.extend_from_slice(&value.to_le_bytes());
        self.written += 8;
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
        self.written += bytes.len();
    }

    pub fn write_fixed<const N: usize>(&mut self, bytes: &[u8; N]) {
        self.buf.extend_from_slice(bytes);
        self.written += N;
    }
}

/// Canonical little-endian decoder with checked bounds.
#[derive(Clone, Debug)]
pub struct LittleEndianDecoder<'a> {
    buf: &'a [u8],
    offset: usize,
}

impl<'a> LittleEndianDecoder<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, offset: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.offset)
    }

    pub fn is_empty(&self) -> bool {
        self.offset >= self.buf.len()
    }

    pub fn position(&self) -> usize {
        self.offset
    }

    pub fn advance(&mut self, n: usize) -> Result<(), PersistenceError> {
        let new_offset = self
            .offset
            .checked_add(n)
            .ok_or_else(|| PersistenceError::codec("offset overflow"))?;
        if new_offset > self.buf.len() {
            return Err(PersistenceError::codec("advance past end of buffer"));
        }
        self.offset = new_offset;
        Ok(())
    }

    pub fn read_u8(&mut self) -> Result<u8, PersistenceError> {
        if self.offset >= self.buf.len() {
            return Err(PersistenceError::codec(
                "unexpected end of buffer reading u8",
            ));
        }
        let value = self.buf[self.offset];
        self.offset += 1;
        Ok(value)
    }

    pub fn read_u16(&mut self) -> Result<u16, PersistenceError> {
        const N: usize = 2;
        if self.offset.saturating_add(N) > self.buf.len() {
            return Err(PersistenceError::codec(
                "unexpected end of buffer reading u16",
            ));
        }
        let value = u16::from_le_bytes(self.buf[self.offset..self.offset + N].try_into().unwrap());
        self.offset += N;
        Ok(value)
    }

    pub fn read_u32(&mut self) -> Result<u32, PersistenceError> {
        const N: usize = 4;
        if self.offset.saturating_add(N) > self.buf.len() {
            return Err(PersistenceError::codec(
                "unexpected end of buffer reading u32",
            ));
        }
        let value = u32::from_le_bytes(self.buf[self.offset..self.offset + N].try_into().unwrap());
        self.offset += N;
        Ok(value)
    }

    pub fn read_u64(&mut self) -> Result<u64, PersistenceError> {
        const N: usize = 8;
        if self.offset.saturating_add(N) > self.buf.len() {
            return Err(PersistenceError::codec(
                "unexpected end of buffer reading u64",
            ));
        }
        let value = u64::from_le_bytes(self.buf[self.offset..self.offset + N].try_into().unwrap());
        self.offset += N;
        Ok(value)
    }

    pub fn read_i64(&mut self) -> Result<i64, PersistenceError> {
        const N: usize = 8;
        if self.offset.saturating_add(N) > self.buf.len() {
            return Err(PersistenceError::codec(
                "unexpected end of buffer reading i64",
            ));
        }
        let value = i64::from_le_bytes(self.buf[self.offset..self.offset + N].try_into().unwrap());
        self.offset += N;
        Ok(value)
    }

    pub fn read_fixed<const N: usize>(&mut self) -> Result<&'a [u8; N], PersistenceError> {
        if self.offset.saturating_add(N) > self.buf.len() {
            return Err(PersistenceError::codec(format!(
                "unexpected end of buffer reading fixed {N} bytes"
            )));
        }
        let bytes = self.buf[self.offset..self.offset + N].try_into().unwrap();
        self.offset += N;
        Ok(bytes)
    }

    pub fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], PersistenceError> {
        let end = self
            .offset
            .checked_add(n)
            .ok_or_else(|| PersistenceError::codec("byte count overflow"))?;
        if end > self.buf.len() {
            return Err(PersistenceError::codec(format!(
                "unexpected end of buffer reading {n} bytes"
            )));
        }
        let bytes = &self.buf[self.offset..end];
        self.offset = end;
        Ok(bytes)
    }

    pub fn slice_from(&self, start: usize, length: usize) -> Result<&'a [u8], PersistenceError> {
        let end = start
            .checked_add(length)
            .ok_or_else(|| PersistenceError::codec("slice bounds overflow"))?;
        if end > self.buf.len() {
            return Err(PersistenceError::codec("slice past end of buffer"));
        }
        Ok(&self.buf[start..end])
    }
}
