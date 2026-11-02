//! World storage and global coordinate spatial queries.

use crate::block::BlockState;
use crate::chunk::Chunk;
use std::collections::HashMap;

/// In-memory Minecraft world storing active chunks.
#[derive(Debug, Default)]
pub struct World {
    chunks: HashMap<(i32, i32), Chunk>,
}

impl World {
    /// Creates a new empty `World`.
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
        }
    }

    /// Inserts or replaces a chunk in the world.
    pub fn insert_chunk(&mut self, chunk: Chunk) {
        self.chunks.insert((chunk.x, chunk.z), chunk);
    }

    /// Removes a chunk from the world (e.g. unloaded chunk).
    pub fn unload_chunk(&mut self, x: i32, z: i32) -> Option<Chunk> {
        self.chunks.remove(&(x, z))
    }

    /// Returns a reference to a chunk at chunk coordinates (x, z).
    pub fn get_chunk(&self, x: i32, z: i32) -> Option<&Chunk> {
        self.chunks.get(&(x, z))
    }

    /// Returns a mutable reference to a chunk at chunk coordinates.
    pub fn get_chunk_mut(&mut self, x: i32, z: i32) -> Option<&mut Chunk> {
        self.chunks.get_mut(&(x, z))
    }

    /// Total number of loaded chunks.
    pub fn loaded_chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Queries the block state at global world coordinates (x, y, z).
    pub fn get_block(&self, x: i32, y: i32, z: i32) -> BlockState {
        if !(0..=255).contains(&y) {
            return BlockState::AIR;
        }

        let chunk_x = x.div_euclid(16);
        let chunk_z = z.div_euclid(16);

        let local_x = x.rem_euclid(16) as usize;
        let local_y = y as usize;
        let local_z = z.rem_euclid(16) as usize;

        match self.get_chunk(chunk_x, chunk_z) {
            Some(chunk) => chunk.get_block(local_x, local_y, local_z),
            None => BlockState::AIR,
        }
    }

    /// Sets a block state at global world coordinates (x, y, z).
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, block: BlockState) {
        if !(0..=255).contains(&y) {
            return;
        }

        let chunk_x = x.div_euclid(16);
        let chunk_z = z.div_euclid(16);

        let local_x = x.rem_euclid(16) as usize;
        let local_y = y as usize;
        let local_z = z.rem_euclid(16) as usize;

        if let Some(chunk) = self.get_chunk_mut(chunk_x, chunk_z) {
            chunk.set_block(local_x, local_y, local_z, block);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_global_block_lookup_negative_coords() {
        let mut world = World::new();
        let chunk = Chunk::new(-1, -1);
        world.insert_chunk(chunk);

        let obsidian = BlockState::new(49, 0);
        // Global (-1, 64, -1) maps to chunk (-1, -1), local (15, 64, 15)
        world.set_block(-1, 64, -1, obsidian);

        assert_eq!(world.get_block(-1, 64, -1), obsidian);
        assert_eq!(world.get_block(-2, 64, -1), BlockState::AIR);
    }
}
