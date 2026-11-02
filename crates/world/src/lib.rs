//! World tracking, chunk storage, and block data for Minecraft 1.8.9.

#![deny(missing_docs)]

pub mod block;
pub mod chunk;
pub mod decoder;
pub mod world;

pub use block::BlockState;
pub use chunk::{Chunk, ChunkSection};
pub use decoder::decode_chunk_data;
pub use world::World;
