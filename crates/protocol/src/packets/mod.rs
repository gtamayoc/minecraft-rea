//! Packets definition for Minecraft 1.8.9 Protocol 47.

pub mod handshake;

pub use handshake::{
    HandshakePacket, LoginStartPacket, LoginSuccessPacket, NextState, SetCompressionPacket,
    PROTOCOL_VERSION,
};
