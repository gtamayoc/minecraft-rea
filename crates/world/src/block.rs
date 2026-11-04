//! Block representation and state for Minecraft 1.8.9.

/// A block represented by an 8-bit ID and 4-bit metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct BlockState {
    /// Minecraft block ID (0-255).
    pub id: u8,
    /// Block metadata/data value (0-15).
    pub metadata: u8,
}

impl BlockState {
    /// Air block (ID 0, metadata 0).
    pub const AIR: Self = Self { id: 0, metadata: 0 };

    /// Creates a new `BlockState`.
    #[inline]
    pub const fn new(id: u8, metadata: u8) -> Self {
        Self {
            id,
            metadata: metadata & 0x0F,
        }
    }

    /// Creates a `BlockState` from a 16-bit block state value (ID << 4 | metadata).
    #[inline]
    pub const fn from_raw(raw: u16) -> Self {
        let id = (raw >> 4) as u8;
        let metadata = (raw & 0x0F) as u8;
        Self { id, metadata }
    }

    /// Converts this block state into a 16-bit raw value.
    #[inline]
    pub const fn to_raw(self) -> u16 {
        ((self.id as u16) << 4) | (self.metadata as u16)
    }

    /// Checks if the block is air (empty space).
    #[inline]
    pub const fn is_air(self) -> bool {
        self.id == 0
    }

    /// Checks if the block is opaque/solid for face culling.
    pub fn is_opaque_cube(self) -> bool {
        match self.id {
            0 => false,              // Air
            8 | 9 => false,          // Water
            10 | 11 => false,        // Lava
            18 | 161 => false,       // Leaves
            20 | 95 => false,        // Glass / Stained Glass
            31 | 32 => false,        // Tallgrass / Dead Bush
            37..=40 => false,        // Flowers / Mushrooms
            50 => false,             // Torch
            51 => false,             // Fire
            65 => false,             // Ladder
            85 | 188..=192 => false, // Fences
            101 | 102 => false,      // Iron Bars / Glass Pane
            _ => true,
        }
    }

    /// Returns a representative RGB color for this block (for clean shader rendering before texture atlas).
    pub fn debug_color(self) -> [f32; 3] {
        match self.id {
            1 => [0.55, 0.55, 0.55],  // Stone (Gray)
            2 => [0.35, 0.65, 0.25],  // Grass (Green)
            3 => [0.50, 0.35, 0.20],  // Dirt (Brown)
            4 => [0.45, 0.45, 0.45],  // Cobblestone
            5 => [0.65, 0.50, 0.30],  // Planks (Wood)
            7 => [0.20, 0.20, 0.20],  // Bedrock (Dark charcoal)
            12 => [0.85, 0.82, 0.60], // Sand (Yellowish)
            13 => [0.48, 0.48, 0.48], // Gravel
            14 => [0.90, 0.80, 0.20], // Gold Ore
            15 => [0.70, 0.55, 0.45], // Iron Ore
            16 => [0.25, 0.25, 0.25], // Coal Ore
            17 => [0.40, 0.30, 0.18], // Log
            18 => [0.20, 0.50, 0.15], // Leaves
            35 => [0.90, 0.90, 0.90], // Wool (White)
            41 => [0.95, 0.85, 0.10], // Gold Block
            42 => [0.85, 0.85, 0.85], // Iron Block
            56 => [0.20, 0.80, 0.85], // Diamond Ore
            57 => [0.30, 0.90, 0.95], // Diamond Block
            _ => [0.60, 0.60, 0.60],  // Default neutral
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_state_raw_conversion() {
        let block = BlockState::new(1, 2); // Stone with variant 2 (polished granite)
        let raw = block.to_raw();
        assert_eq!(raw, (1 << 4) | 2);

        let decoded = BlockState::from_raw(raw);
        assert_eq!(decoded, block);
    }

    #[test]
    fn air_is_not_opaque() {
        assert!(!BlockState::AIR.is_opaque_cube());
        assert!(BlockState::AIR.is_air());
    }

    #[test]
    fn stone_is_opaque() {
        let stone = BlockState::new(1, 0);
        assert!(stone.is_opaque_cube());
    }
}
