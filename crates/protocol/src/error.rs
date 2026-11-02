//! Error types for protocol encoding, decoding, and connection handling.

use thiserror::Error;

/// Errors that can occur during packet parsing and serialization.
#[derive(Debug, Error)]
pub enum ProtocolError {
    /// VarInt exceeds maximum byte length (5 bytes for 32-bit integer).
    #[error("VarInt exceeds maximum allowed size of 5 bytes")]
    VarIntTooLarge,

    /// VarLong exceeds maximum byte length (10 bytes for 64-bit integer).
    #[error("VarLong exceeds maximum allowed size of 10 bytes")]
    VarLongTooLarge,

    /// String exceeds maximum allowed length.
    #[error("String length {0} exceeds max allowed {1}")]
    StringTooLong(usize, usize),

    /// Invalid UTF-8 sequence in string.
    #[error("Invalid UTF-8 sequence in packet string: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),

    /// Packet frame length is invalid or exceeds buffer limits.
    #[error("Packet frame length {0} is invalid")]
    InvalidFrameLength(i32),

    /// Unexpected end of stream or buffer when reading data.
    #[error("Unexpected end of data: needed {needed} bytes, got {available}")]
    UnexpectedEof {
        /// Number of bytes needed.
        needed: usize,
        /// Number of bytes actually available.
        available: usize,
    },

    /// Unrecognized packet ID for the current connection state.
    #[error("Unknown packet ID 0x{packet_id:02X} for state {state}")]
    UnknownPacketId {
        /// ID of the unknown packet.
        packet_id: i32,
        /// Connection state name.
        state: &'static str,
    },

    /// Zlib decompression failure.
    #[error("Decompression failed: {0}")]
    DecompressionError(#[from] std::io::Error),
}

/// Standard Result alias for protocol operations.
pub type Result<T> = std::result::Result<T, ProtocolError>;
