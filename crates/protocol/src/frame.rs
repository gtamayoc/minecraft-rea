//! Protocol state machine and packet framing codec.

use crate::error::{ProtocolError, Result};
use crate::types::varint::{read_varint, varint_size, write_varint};
use bytes::{Buf, Bytes, BytesMut};

/// Connection state of the client according to Minecraft 1.8 Protocol 47.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ConnectionState {
    /// Initial state: Client sends Handshake packet.
    #[default]
    Handshaking,
    /// Status ping/query state.
    Status,
    /// Authentication and login sequence.
    Login,
    /// Main game loop and world streaming state.
    Play,
}

impl ConnectionState {
    /// Returns the human-readable state name.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Handshaking => "Handshaking",
            Self::Status => "Status",
            Self::Login => "Login",
            Self::Play => "Play",
        }
    }
}

/// A raw decoded packet frame containing the packet ID and payload bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawPacket {
    /// Numeric packet identifier.
    pub id: i32,
    /// Uncompressed payload of the packet.
    pub payload: Bytes,
}

impl RawPacket {
    /// Creates a new `RawPacket`.
    #[inline]
    pub fn new(id: i32, payload: Bytes) -> Self {
        Self { id, payload }
    }

    /// Encodes this packet into a frame with length prefix (uncompressed format).
    pub fn encode_uncompressed(&self) -> BytesMut {
        let id_size = varint_size(self.id);
        let frame_len = id_size + self.payload.len();
        let total_size = varint_size(frame_len as i32) + frame_len;

        let mut buf = BytesMut::with_capacity(total_size);
        write_varint(&mut buf, frame_len as i32);
        write_varint(&mut buf, self.id);
        buf.extend_from_slice(&self.payload);
        buf
    }
}

/// Helper codec for decoding uncompressed packet frames from an incoming stream buffer.
pub struct FrameDecoder;

impl FrameDecoder {
    /// Attempts to decode one `RawPacket` from the buffer without panicking.
    ///
    /// If the buffer does not contain a complete packet yet, returns `Ok(None)` and leaves
    /// the buffer untouched so more data can be read.
    pub fn decode(src: &mut BytesMut) -> Result<Option<RawPacket>> {
        if src.is_empty() {
            return Ok(None);
        }

        let mut temp_cursor = std::io::Cursor::new(&src[..]);
        let packet_length = match read_varint(&mut temp_cursor) {
            Ok(len) => len,
            Err(ProtocolError::UnexpectedEof { .. }) => return Ok(None),
            Err(e) => return Err(e),
        };

        if packet_length <= 0 {
            return Err(ProtocolError::InvalidFrameLength(packet_length));
        }

        let length_varint_size = temp_cursor.position() as usize;
        let packet_len_usize = packet_length as usize;

        // Ensure the full frame payload has arrived
        if src.len() < length_varint_size + packet_len_usize {
            return Ok(None);
        }

        // Advance buffer past the length prefix
        src.advance(length_varint_size);

        // Split out exactly the packet data
        let mut packet_data = src.split_to(packet_len_usize);

        // Read packet ID from packet data
        let packet_id = read_varint(&mut packet_data)?;
        let payload = packet_data.freeze();

        Ok(Some(RawPacket::new(packet_id, payload)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_packet_encode_decode_roundtrip() {
        let payload_bytes = Bytes::from_static(b"Hello Minecraft World");
        let packet = RawPacket::new(0x01, payload_bytes.clone());

        let mut encoded = packet.encode_uncompressed();

        let decoded = FrameDecoder::decode(&mut encoded).unwrap();
        assert!(decoded.is_some());
        let decoded = decoded.unwrap();

        assert_eq!(decoded.id, 0x01);
        assert_eq!(decoded.payload, payload_bytes);
        assert!(encoded.is_empty());
    }

    #[test]
    fn incomplete_buffer_returns_none() {
        let payload_bytes = Bytes::from_static(b"Partial Packet");
        let packet = RawPacket::new(0x02, payload_bytes);
        let mut encoded = packet.encode_uncompressed();

        // Truncate encoded buffer by 2 bytes
        let len = encoded.len();
        let mut partial = encoded.split_to(len - 2);

        let decoded = FrameDecoder::decode(&mut partial).unwrap();
        assert!(decoded.is_none());
    }
}
