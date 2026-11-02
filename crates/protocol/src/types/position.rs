//! Block coordinates (`Position`) and string primitives for Minecraft Protocol 47.

use crate::error::{ProtocolError, Result};
use crate::types::varint::{read_varint, varint_size, write_varint};
use bytes::{Buf, BufMut};

/// Maximum allowed string length in characters (per protocol 47 specification).
pub const MAX_STRING_LENGTH: usize = 32767;

/// A 3D block coordinate packed into a 64-bit unsigned integer.
///
/// In Minecraft 1.8 Protocol 47:
/// - X: 26 bits signed (values from -33,554,432 to 33,554,431)
/// - Y: 12 bits unsigned (values from 0 to 4095)
/// - Z: 26 bits signed (values from -33,554,432 to 33,554,431)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    /// X block coordinate.
    pub x: i32,
    /// Y block coordinate (height).
    pub y: i32,
    /// Z block coordinate.
    pub z: i32,
}

impl Position {
    /// Creates a new `Position`.
    #[inline]
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Encodes the position into a 64-bit integer.
    #[inline]
    pub fn to_u64(self) -> u64 {
        let x_bits = (self.x as i64 & 0x3FFFFFF) as u64;
        let y_bits = (self.y as i64 & 0xFFF) as u64;
        let z_bits = (self.z as i64 & 0x3FFFFFF) as u64;

        (x_bits << 38) | (y_bits << 26) | z_bits
    }

    /// Decodes a `Position` from a 64-bit packed integer with arithmetic sign preservation.
    #[inline]
    pub fn from_u64(val: u64) -> Self {
        // Shift left then shift right arithmetically to preserve negative signs
        let x = ((val as i64) >> 38) as i32;
        let y = (((val >> 26) & 0xFFF) as i64) as i32;
        // Shift left by 38 to push 26 bits to MSB, then shift right arithmetically
        let z = (((val as i64) << 38) >> 38) as i32;

        Self { x, y, z }
    }
}

/// Reads a `Position` from a buffer.
pub fn read_position<B: Buf>(buf: &mut B) -> Result<Position> {
    if buf.remaining() < 8 {
        return Err(ProtocolError::UnexpectedEof {
            needed: 8,
            available: buf.remaining(),
        });
    }
    let val = buf.get_u64();
    Ok(Position::from_u64(val))
}

/// Writes a `Position` to a buffer.
pub fn write_position<B: BufMut>(buf: &mut B, pos: Position) {
    buf.put_u64(pos.to_u64());
}

/// Reads a UTF-8 string prefixed with a `VarInt` length.
pub fn read_string<B: Buf>(buf: &mut B, max_len: usize) -> Result<String> {
    let byte_len = read_varint(buf)? as usize;
    if byte_len > max_len * 4 {
        return Err(ProtocolError::StringTooLong(byte_len, max_len * 4));
    }

    if buf.remaining() < byte_len {
        return Err(ProtocolError::UnexpectedEof {
            needed: byte_len,
            available: buf.remaining(),
        });
    }

    let mut bytes = vec![0u8; byte_len];
    buf.copy_to_slice(&mut bytes);

    let s = String::from_utf8(bytes)?;
    if s.chars().count() > max_len {
        return Err(ProtocolError::StringTooLong(s.chars().count(), max_len));
    }

    Ok(s)
}

/// Writes a UTF-8 string prefixed with its byte length as a `VarInt`.
pub fn write_string<B: BufMut>(buf: &mut B, s: &str) {
    let bytes = s.as_bytes();
    write_varint(buf, bytes.len() as i32);
    buf.put_slice(bytes);
}

/// Computes the serialized size of a string.
#[inline]
pub fn string_size(s: &str) -> usize {
    let len = s.len();
    varint_size(len as i32) + len
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn position_roundtrip_positive() {
        let original = Position::new(100, 64, 250);
        let mut buf = BytesMut::new();
        write_position(&mut buf, original);
        assert_eq!(buf.len(), 8);
        let decoded = read_position(&mut buf).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn position_roundtrip_negative_coordinates() {
        let test_cases = [
            Position::new(-1, 0, -1),
            Position::new(-12345, 128, -67890),
            Position::new(-33554432, 255, -33554432),
            Position::new(33554431, 0, 33554431),
        ];

        for pos in test_cases {
            let mut buf = BytesMut::new();
            write_position(&mut buf, pos);
            let decoded = read_position(&mut buf).unwrap();
            assert_eq!(decoded, pos, "Failed for {:?}", pos);
        }
    }

    #[test]
    fn string_roundtrip_utf8() {
        let msg = "¡Hola Minecraft 1.8 en Rust! 🚀";
        let mut buf = BytesMut::new();
        write_string(&mut buf, msg);
        let decoded = read_string(&mut buf, MAX_STRING_LENGTH).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn string_enforces_max_length() {
        let msg = "Too long string for test";
        let mut buf = BytesMut::new();
        write_string(&mut buf, msg);
        let res = read_string(&mut buf, 5);
        assert!(matches!(res, Err(ProtocolError::StringTooLong(_, _))));
    }
}
