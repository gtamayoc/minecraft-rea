//! Decoder for Minecraft 1.8.9 S21 Chunk Data packets.

use crate::block::BlockState;
use crate::chunk::{Chunk, ChunkSection, BLOCKS_PER_SECTION, NUM_SECTIONS};
use bytes::Buf;
use minecraft_protocol::error::{ProtocolError, Result};
use minecraft_protocol::types::varint::read_varint;

/// Decodes an S21 Chunk Data packet (ID 0x21 in Play state) into an updated or new `Chunk`.
///
/// In Minecraft 1.8 Protocol 47:
/// - `chunk_x`: i32
/// - `chunk_z`: i32
/// - `ground_up_continuous`: bool
/// - `primary_bit_mask`: u16 (bit N is 1 if section N is included)
/// - `size`: VarInt (length of data in bytes)
/// - `data`: byte array containing section block types, metadata, block light, sky light, biomes
pub fn decode_chunk_data<B: Buf>(buf: &mut B) -> Result<Chunk> {
    if buf.remaining() < 10 {
        return Err(ProtocolError::UnexpectedEof {
            needed: 10,
            available: buf.remaining(),
        });
    }

    let chunk_x = buf.get_i32();
    let chunk_z = buf.get_i32();
    let ground_up_continuous = buf.get_u8() != 0;
    let primary_bit_mask = buf.get_u16();
    let data_len = read_varint(buf)? as usize;

    if buf.remaining() < data_len {
        return Err(ProtocolError::UnexpectedEof {
            needed: data_len,
            available: buf.remaining(),
        });
    }

    let mut chunk = Chunk::new(chunk_x, chunk_z);
    let mut data = buf.take(data_len);

    // 1. Read block IDs and metadata for each included section
    // Each block is stored as a 16-bit little-endian number: (block_id << 4) | metadata
    for section_idx in 0..NUM_SECTIONS {
        if (primary_bit_mask & (1 << section_idx)) != 0 {
            let section = chunk.sections[section_idx].get_or_insert_with(ChunkSection::new);

            for block_idx in 0..BLOCKS_PER_SECTION {
                if data.remaining() < 2 {
                    break;
                }
                // Minecraft 1.8 stores block states in chunk packets as Little Endian u16
                let raw_block = data.get_u16_le();
                section.blocks[block_idx] = BlockState::from_raw(raw_block);
            }
        }
    }

    // 2. Read block light arrays (2048 bytes per included section)
    for section_idx in 0..NUM_SECTIONS {
        if (primary_bit_mask & (1 << section_idx)) != 0 {
            if let Some(section) = &mut chunk.sections[section_idx] {
                if data.remaining() >= 2048 {
                    data.copy_to_slice(&mut section.block_light[..]);
                }
            }
        }
    }

    // 3. Read sky light arrays (2048 bytes per included section)
    for section_idx in 0..NUM_SECTIONS {
        if (primary_bit_mask & (1 << section_idx)) != 0 {
            if let Some(section) = &mut chunk.sections[section_idx] {
                if data.remaining() >= 2048 {
                    data.copy_to_slice(&mut section.sky_light[..]);
                }
            }
        }
    }

    // 4. If ground_up_continuous is true, read the 256 biome bytes
    if ground_up_continuous && data.remaining() >= 256 {
        data.copy_to_slice(&mut chunk.biomes);
    }

    Ok(chunk)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::{BufMut, BytesMut};

    #[test]
    fn decode_single_section_chunk() {
        let mut buf = BytesMut::new();
        buf.put_i32(0); // chunk_x
        buf.put_i32(0); // chunk_z
        buf.put_u8(1); // ground_up_continuous
        buf.put_u16(1); // bitmask = section 0 only

        // Payload data size:
        // Blocks: 4096 * 2 bytes = 8192
        // BlockLight: 2048 bytes
        // SkyLight: 2048 bytes
        // Biomes: 256 bytes
        // Total = 12544 bytes
        let total_payload: i32 = 4096 * 2 + 2048 + 2048 + 256;
        minecraft_protocol::types::varint::write_varint(&mut buf, total_payload);

        // Blocks: Fill first block with stone (ID 1, metadata 0)
        buf.put_u16_le(1 << 4);
        // Fill the remaining 4095 blocks with air
        for _ in 1..4096 {
            buf.put_u16_le(0);
        }

        // Blocklight (2048 zeros)
        buf.put_bytes(0, 2048);
        // Skylight (2048 ones)
        buf.put_bytes(0xFF, 2048);
        // Biomes (256 plains: biome ID 1)
        buf.put_bytes(1, 256);

        let chunk = decode_chunk_data(&mut buf).unwrap();
        assert_eq!(chunk.x, 0);
        assert_eq!(chunk.z, 0);
        assert_eq!(chunk.get_block(0, 0, 0), BlockState::new(1, 0));
        assert_eq!(chunk.get_block(1, 0, 0), BlockState::AIR);
        assert_eq!(chunk.biomes[0], 1);
    }
}
