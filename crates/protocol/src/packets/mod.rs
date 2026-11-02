//! Packets definition for Minecraft 1.8.9 Protocol 47.

pub mod handshake;
pub mod play;

pub use handshake::{
    HandshakePacket, LoginStartPacket, LoginSuccessPacket, NextState, SetCompressionPacket,
    PROTOCOL_VERSION,
};
pub use play::{
    ClientKeepAlivePacket, ClientPlayerPositionAndLookPacket, JoinGamePacket,
    PlayerPositionAndLookPacket, ServerChatMessagePacket, ServerKeepAlivePacket,
    SpawnPositionPacket,
};
