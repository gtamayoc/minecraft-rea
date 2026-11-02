//! Handshake and Login packets for Minecraft Protocol 47.

use crate::error::Result;
use crate::frame::RawPacket;
use crate::types::position::{read_string, write_string};
use crate::types::varint::{read_varint, write_varint};
use bytes::{Buf, Bytes, BytesMut};

/// Protocol number for Minecraft Java 1.8.9.
pub const PROTOCOL_VERSION: i32 = 47;

/// Next state requested after Handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NextState {
    /// Request status ping (server list ping).
    Status = 1,
    /// Request login (play on server).
    Login = 2,
}

impl NextState {
    /// Converts numeric representation to `NextState`.
    pub fn from_i32(val: i32) -> Option<Self> {
        match val {
            1 => Some(Self::Status),
            2 => Some(Self::Login),
            _ => None,
        }
    }
}

/// Handshake packet sent by the client (ID 0x00).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakePacket {
    /// Protocol version (47 for 1.8.9).
    pub protocol_version: i32,
    /// Server hostname or IP address.
    pub server_address: String,
    /// Server port (usually 25565).
    pub server_port: u16,
    /// Intended next state (1 for status, 2 for login).
    pub next_state: NextState,
}

impl HandshakePacket {
    /// Creates a standard Handshake targeting a server with Protocol 47.
    pub fn new(server_address: impl Into<String>, server_port: u16, next_state: NextState) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            server_address: server_address.into(),
            server_port,
            next_state,
        }
    }

    /// Serializes this packet into a `RawPacket`.
    pub fn to_raw_packet(&self) -> RawPacket {
        let mut payload = BytesMut::new();
        write_varint(&mut payload, self.protocol_version);
        write_string(&mut payload, &self.server_address);
        payload.extend_from_slice(&self.server_port.to_be_bytes());
        write_varint(&mut payload, self.next_state as i32);

        RawPacket::new(0x00, payload.freeze())
    }

    /// Deserializes a `HandshakePacket` from raw payload bytes.
    pub fn from_payload(mut payload: Bytes) -> Result<Self> {
        let protocol_version = read_varint(&mut payload)?;
        let server_address = read_string(&mut payload, 255)?;
        let server_port = payload.get_u16();
        let next_state_raw = read_varint(&mut payload)?;
        let next_state = NextState::from_i32(next_state_raw).unwrap_or(NextState::Login);

        Ok(Self {
            protocol_version,
            server_address,
            server_port,
            next_state,
        })
    }
}

/// Login Start packet sent by the client (ID 0x00 in Login state).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginStartPacket {
    /// Player username (max 16 characters).
    pub username: String,
}

impl LoginStartPacket {
    /// Creates a new `LoginStartPacket`.
    pub fn new(username: impl Into<String>) -> Self {
        Self {
            username: username.into(),
        }
    }

    /// Serializes this packet into a `RawPacket`.
    pub fn to_raw_packet(&self) -> RawPacket {
        let mut payload = BytesMut::new();
        write_string(&mut payload, &self.username);
        RawPacket::new(0x00, payload.freeze())
    }

    /// Deserializes from raw payload bytes.
    pub fn from_payload(mut payload: Bytes) -> Result<Self> {
        let username = read_string(&mut payload, 16)?;
        Ok(Self { username })
    }
}

/// Set Compression packet sent by the server during login (ID 0x03).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetCompressionPacket {
    /// Threshold in bytes over which packets are compressed.
    pub threshold: i32,
}

impl SetCompressionPacket {
    /// Deserializes from raw payload bytes.
    pub fn from_payload(mut payload: Bytes) -> Result<Self> {
        let threshold = read_varint(&mut payload)?;
        Ok(Self { threshold })
    }
}

/// Login Success packet sent by the server (ID 0x02 in Login state).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginSuccessPacket {
    /// Player UUID as a string.
    pub uuid: String,
    /// Player username.
    pub username: String,
}

impl LoginSuccessPacket {
    /// Deserializes from raw payload bytes.
    pub fn from_payload(mut payload: Bytes) -> Result<Self> {
        let uuid = read_string(&mut payload, 36)?;
        let username = read_string(&mut payload, 16)?;
        Ok(Self { uuid, username })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_packet_roundtrip() {
        let packet = HandshakePacket::new("localhost", 25565, NextState::Login);
        let raw = packet.to_raw_packet();
        assert_eq!(raw.id, 0x00);

        let decoded = HandshakePacket::from_payload(raw.payload).unwrap();
        assert_eq!(decoded, packet);
    }

    #[test]
    fn login_start_roundtrip() {
        let packet = LoginStartPacket::new("RustPlayer");
        let raw = packet.to_raw_packet();
        assert_eq!(raw.id, 0x00);

        let decoded = LoginStartPacket::from_payload(raw.payload).unwrap();
        assert_eq!(decoded.username, "RustPlayer");
    }

    #[test]
    fn login_success_decode() {
        let mut payload = BytesMut::new();
        write_string(&mut payload, "c86e08c8-b5b4-4b5f-9e6b-67e3bf5d4e1a");
        write_string(&mut payload, "RustPlayer");

        let decoded = LoginSuccessPacket::from_payload(payload.freeze()).unwrap();
        assert_eq!(decoded.uuid, "c86e08c8-b5b4-4b5f-9e6b-67e3bf5d4e1a");
        assert_eq!(decoded.username, "RustPlayer");
    }
}
