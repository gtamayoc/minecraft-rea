//! Variable-length integer implementations (`VarInt` and `VarLong`) for Protocol 47.
//!
//! Uses LEB128 encoding where the 7 least significant bits store payload
//! and bit 7 (0x80) indicates whether subsequent bytes follow.

use crate::error::{ProtocolError, Result};
use bytes::{Buf, BufMut};

/// Maximum byte count for a 32-bit VarInt.
pub const MAX_VARINT_BYTES: usize = 5;

/// Maximum byte count for a 64-bit VarLong.
pub const MAX_VARLONG_BYTES: usize = 10;

/// Reads a 32-bit signed `VarInt` from a byte buffer.
pub fn read_varint<B: Buf>(buf: &mut B) -> Result<i32> {
    let mut value: i32 = 0;
    let mut position: u32 = 0;

    for byte_count in 0..MAX_VARINT_BYTES {
        if !buf.has_remaining() {
            return Err(ProtocolError::UnexpectedEof {
                needed: 1,
                available: 0,
            });
        }

        let current_byte = buf.get_u8();
        value |= ((current_byte & 0x7F) as i32) << position;

        // If the MSB is not set, no further bytes follow
        if (current_byte & 0x80) == 0 {
            return Ok(value);
        }

        position += 7;
        if byte_count == MAX_VARINT_BYTES - 1 {
            return Err(ProtocolError::VarIntTooLarge);
        }
    }

    Err(ProtocolError::VarIntTooLarge)
}

/// Writes a 32-bit signed `VarInt` to a buffer.
pub fn write_varint<B: BufMut>(buf: &mut B, mut value: i32) {
    loop {
        // If remaining bits fit in 7 bits, write last byte without MSB set
        if (value as u32 & !0x7F) == 0 {
            buf.put_u8(value as u8);
            return;
        }

        buf.put_u8(((value & 0x7F) | 0x80) as u8);
        value = ((value as u32) >> 7) as i32;
    }
}

/// Computes the exact encoded size in bytes of a `VarInt` without writing it.
#[inline]
pub fn varint_size(value: i32) -> usize {
    let mut v = value as u32;
    let mut size = 0;
    loop {
        size += 1;
        if (v & !0x7F) == 0 {
            return size;
        }
        v >>= 7;
    }
}

/// Reads a 64-bit signed `VarLong` from a byte buffer.
pub fn read_varlong<B: Buf>(buf: &mut B) -> Result<i64> {
    let mut value: i64 = 0;
    let mut position: u32 = 0;

    for byte_count in 0..MAX_VARLONG_BYTES {
        if !buf.has_remaining() {
            return Err(ProtocolError::UnexpectedEof {
                needed: 1,
                available: 0,
            });
        }

        let current_byte = buf.get_u8();
        value |= ((current_byte & 0x7F) as i64) << position;

        if (current_byte & 0x80) == 0 {
            return Ok(value);
        }

        position += 7;
        if byte_count == MAX_VARLONG_BYTES - 1 {
            return Err(ProtocolError::VarLongTooLarge);
        }
    }

    Err(ProtocolError::VarLongTooLarge)
}

/// Writes a 64-bit signed `VarLong` to a buffer.
pub fn write_varlong<B: BufMut>(buf: &mut B, mut value: i64) {
    loop {
        if (value as u64 & !0x7F) == 0 {
            buf.put_u8(value as u8);
            return;
        }

        buf.put_u8(((value & 0x7F) | 0x80) as u8);
        value = ((value as u64) >> 7) as i64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn varint_roundtrip_zero() {
        let mut buf = BytesMut::new();
        write_varint(&mut buf, 0);
        assert_eq!(buf.len(), 1);
        let read = read_varint(&mut buf).unwrap();
        assert_eq!(read, 0);
    }

    #[test]
    fn varint_roundtrip_positive() {
        let values = [1, 2, 127, 128, 255, 25565, 2147483647];
        for val in values {
            let mut buf = BytesMut::new();
            write_varint(&mut buf, val);
            assert_eq!(buf.len(), varint_size(val));
            let read = read_varint(&mut buf).unwrap();
            assert_eq!(read, val);
        }
    }

    #[test]
    fn varint_roundtrip_negative() {
        let values = [-1, -2, -128, -2147483648];
        for val in values {
            let mut buf = BytesMut::new();
            write_varint(&mut buf, val);
            assert_eq!(buf.len(), varint_size(val));
            let read = read_varint(&mut buf).unwrap();
            assert_eq!(read, val);
        }
    }

    #[test]
    fn varint_overflow_returns_error() {
        // 6 bytes with MSB set is invalid for a 32-bit VarInt
        let raw = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01];
        let mut cursor = std::io::Cursor::new(&raw[..]);
        let res = read_varint(&mut cursor);
        assert!(matches!(res, Err(ProtocolError::VarIntTooLarge)));
    }

    #[test]
    fn varlong_roundtrip_extreme_values() {
        let values = [0, 1, -1, i64::MAX, i64::MIN];
        for val in values {
            let mut buf = BytesMut::new();
            write_varlong(&mut buf, val);
            let read = read_varlong(&mut buf).unwrap();
            assert_eq!(read, val);
        }
    }
}
