//! Byte reader for Verus request serialization (`bufferutils` in
//! verus-typescript-primitives). Every read is bounds-checked; a short or
//! malformed buffer fails as an invalid request.

use crate::types::WalletError;

pub(crate) fn invalid() -> WalletError {
    WalletError::GenericRequestInvalidEnvelope
}

#[derive(Debug, Clone)]
pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    pub(crate) fn position(&self) -> usize {
        self.pos
    }

    pub(crate) fn remaining(&self) -> usize {
        self.bytes.len() - self.pos
    }

    pub(crate) fn is_done(&self) -> bool {
        self.pos == self.bytes.len()
    }

    /// Fails unless every byte was read. Upstream rejects trailing bytes in a
    /// detail's payload since `3ffa9d77`.
    pub(crate) fn finish(&self) -> Result<(), WalletError> {
        if self.is_done() {
            Ok(())
        } else {
            Err(invalid())
        }
    }

    pub(crate) fn take(&mut self, len: usize) -> Result<&'a [u8], WalletError> {
        let end = self.pos.checked_add(len).ok_or_else(invalid)?;
        let slice = self.bytes.get(self.pos..end).ok_or_else(invalid)?;
        self.pos = end;
        Ok(slice)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, WalletError> {
        Ok(self.take(1)?[0])
    }

    pub(crate) fn u32_le(&mut self) -> Result<u32, WalletError> {
        let bytes: [u8; 4] = self.take(4)?.try_into().map_err(|_| invalid())?;
        Ok(u32::from_le_bytes(bytes))
    }

    pub(crate) fn i64_le(&mut self) -> Result<i64, WalletError> {
        let bytes: [u8; 8] = self.take(8)?.try_into().map_err(|_| invalid())?;
        Ok(i64::from_le_bytes(bytes))
    }

    pub(crate) fn compact_size(&mut self) -> Result<u64, WalletError> {
        let first = self.u8()?;
        Ok(match first {
            0..=252 => u64::from(first),
            253 => u64::from(u16::from_le_bytes(
                self.take(2)?.try_into().map_err(|_| invalid())?,
            )),
            254 => u64::from(self.u32_le()?),
            255 => u64::from_le_bytes(self.take(8)?.try_into().map_err(|_| invalid())?),
        })
    }

    /// A length or item count. Each item takes at least one byte, so a count
    /// larger than the bytes left is malformed.
    pub(crate) fn count(&mut self) -> Result<usize, WalletError> {
        let value = self.compact_size()?;
        if value > self.remaining() as u64 {
            return Err(invalid());
        }
        Ok(value as usize)
    }

    /// Bitcoin `CVarInt`, as `varint.ts` reads it.
    pub(crate) fn varint(&mut self) -> Result<u64, WalletError> {
        let mut value = 0u64;
        loop {
            let byte = self.u8()?;
            if value > (u64::MAX >> 7) {
                return Err(invalid());
            }
            value = (value << 7) | u64::from(byte & 0x7f);
            if byte & 0x80 == 0 {
                return Ok(value);
            }
            value = value.checked_add(1).ok_or_else(invalid)?;
        }
    }

    pub(crate) fn var_slice(&mut self) -> Result<&'a [u8], WalletError> {
        let len = self.count()?;
        self.take(len)
    }

    pub(crate) fn hash160(&mut self) -> Result<[u8; 20], WalletError> {
        self.take(20)?.try_into().map_err(|_| invalid())
    }

    pub(crate) fn hash256(&mut self) -> Result<[u8; 32], WalletError> {
        self.take(32)?.try_into().map_err(|_| invalid())
    }

    pub(crate) fn utf8(&mut self) -> Result<String, WalletError> {
        let bytes = self.var_slice()?;
        String::from_utf8(bytes.to_vec()).map_err(|_| invalid())
    }

    /// The bytes read since `start`.
    pub(crate) fn since(&self, start: usize) -> &'a [u8] {
        &self.bytes[start..self.pos]
    }
}

#[cfg(test)]
mod tests {
    use super::Reader;
    use crate::core::crypto::verus_id_signature::{write_compact_size, write_varint};

    #[test]
    fn varint_round_trips_multi_byte_values() {
        for value in [
            0u64,
            127,
            128,
            255,
            16_383,
            16_384,
            1_270_500,
            u32::MAX as u64,
        ] {
            let mut out = Vec::new();
            write_varint(&mut out, value);
            let mut reader = Reader::new(&out);
            assert_eq!(reader.varint().unwrap(), value);
            assert!(reader.is_done());
        }
    }

    #[test]
    fn compact_size_reads_each_width() {
        for value in [0usize, 252, 253, 65_535, 65_536] {
            let mut out = Vec::new();
            write_compact_size(&mut out, value);
            assert_eq!(Reader::new(&out).compact_size().unwrap(), value as u64);
        }
    }

    #[test]
    fn counts_larger_than_the_buffer_are_rejected() {
        let mut reader = Reader::new(&[5, 1, 2]);
        assert!(reader.count().is_err());
    }
}
