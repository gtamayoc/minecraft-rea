//! Chunk and ChunkSection data structures for Minecraft 1.8.9.

use crate::block::BlockState;

/// Number of blocks along one horizontal axis of a chunk.
pub const CHUNK_SIZE_X: usize = 16;
/// Number of blocks along the other horizontal axis of a chunk.
pub const CHUNK_SIZE_Z: usize = 16;
/// Height of a single chunk section.
pub const SECTION_HEIGHT: usize = 16;
/// Total number of sections in a vertical chunk column (Y = 0 to 255).
pub const NUM_SECTIONS: usize = 16;
/// Total number of blocks in a 16x16x16 section.
pub const BLOCKS_PER_SECTION: usize = CHUNK_SIZE_X * CHUNK_SIZE_Z * SECTION_HEIGHT; // 4096

/// A 16x16x16 cube of blocks.
#[derive(Debug, Clone)]
pub struct ChunkSection {
    /// 4096 block states indexed by `y * 256 + z * 16 + x`.
    pub blocks: Box<[BlockState; BLOCKS_PER_SECTION]>,
    /// Block light level per block (4 bits per block, 2048 bytes).
    pub block_light: Box<[u8; BLOCKS_PER_SECTION / 2]>,
    /// Sky light level per block (4 bits per block, 2048 bytes).
    pub sky_light: Box<[u8; BLOCKS_PER_SECTION / 2]>,
}

impl Default for ChunkSection {
    fn default() -> Self {
        Self {
            blocks: Box::new([BlockState::AIR; BLOCKS_PER_SECTION]),
            block_light: Box::new([0u8; BLOCKS_PER_SECTION / 2]),
            sky_light: Box::new([0xFFu8; BLOCKS_PER_SECTION / 2]), // Full skylight default
        }
    }
}

impl ChunkSection {
    /// Creates a new empty chunk section.
    pub fn new() -> Self {
        Self::default()
    }

    /// Computes the internal linear index for a coordinate within a section.
    #[inline]
    pub const fn index(x: usize, y: usize, z: usize) -> usize {
        (y * 16 * 16) + (z * 16) + x
    }

    /// Gets the block state at local section coordinates (0-15, 0-15, 0-15).
    #[inline]
    pub fn get_block(&self, x: usize, y: usize, z: usize) -> BlockState {
        if x < 16 && y < 16 && z < 16 {
            self.blocks[Self::index(x, y, z)]
        } else {
            BlockState::AIR
        }
    }

    /// Sets the block state at local section coordinates.
    #[inline]
    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block: BlockState) {
        if x < 16 && y < 16 && z < 16 {
            let idx = Self::index(x, y, z);
            self.blocks[idx] = block;
        }
    }
}

/// A vertical chunk column (16x256x16 blocks).
#[derive(Debug, Clone)]
pub struct Chunk {
    /// Chunk X coordinate in chunk units.
    pub x: i32,
    /// Chunk Z coordinate in chunk units.
    pub z: i32,
    /// Up to 16 sections (indexed 0 to 15 corresponding to Y = 0..16, 16..32, etc.).
    pub sections: [Option<ChunkSection>; NUM_SECTIONS],
    /// 256 bytes biome array (one byte per X,Z column).
    pub biomes: [u8; 256],
}

impl Chunk {
    /// Creates an empty chunk column.
    pub fn new(x: i32, z: i32) -> Self {
        Self {
            x,
            z,
            sections: Default::default(),
            biomes: [0; 256],
        }
    }

    /// Gets a block at local chunk coordinates: x (0..16), y (0..256), z (0..16).
    pub fn get_block(&self, x: usize, y: usize, z: usize) -> BlockState {
        if x >= 16 || y >= 256 || z >= 16 {
            return BlockState::AIR;
        }

        let section_idx = y / 16;
        let local_y = y % 16;

        match &self.sections[section_idx] {
            Some(section) => section.get_block(x, local_y, z),
            None => BlockState::AIR,
        }
    }

    /// Sets a block at local chunk coordinates.
    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block: BlockState) {
        if x >= 16 || y >= 256 || z >= 16 {
            return;
        }

        let section_idx = y / 16;
        let local_y = y % 16;

        let section = self.sections[section_idx].get_or_insert_with(ChunkSection::new);
        section.set_block(x, local_y, z, block);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_section_indexing() {
        let mut section = ChunkSection::new();
        let stone = BlockState::new(1, 0);

        section.set_block(5, 10, 15, stone);
        assert_eq!(section.get_block(5, 10, 15), stone);
        assert_eq!(section.get_block(0, 0, 0), BlockState::AIR);
    }

    #[test]
    fn chunk_get_set_across_sections() {
        let mut chunk = Chunk::new(0, 0);
        let diamond_block = BlockState::new(57, 0);

        // Y = 65 is section 4 (65 / 16 = 4, local y = 1)
        chunk.set_block(3, 65, 7, diamond_block);

        assert_eq!(chunk.get_block(3, 65, 7), diamond_block);
        assert_eq!(chunk.get_block(3, 64, 7), BlockState::AIR);
    }
}
