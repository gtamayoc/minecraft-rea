//! Culled Face Mesher: generates 3D polygon geometry for chunks,
//! discarding all faces between contiguous opaque/solid blocks.

use crate::vertex::{BlockVertex, ChunkMesh};
use glam::{Vec2, Vec3};
use minecraft_world::{BlockState, Chunk, World};

/// Cube face orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockFace {
    /// Top (+Y)
    Top,
    /// Bottom (-Y)
    Bottom,
    /// North (-Z)
    North,
    /// South (+Z)
    South,
    /// West (-X)
    West,
    /// East (+X)
    East,
}

impl BlockFace {
    /// Normal direction vector for this face.
    pub const fn normal(self) -> Vec3 {
        match self {
            Self::Top => Vec3::Y,
            Self::Bottom => Vec3::NEG_Y,
            Self::North => Vec3::NEG_Z,
            Self::South => Vec3::Z,
            Self::West => Vec3::NEG_X,
            Self::East => Vec3::X,
        }
    }

    /// Neighbor offset in block coordinates (dx, dy, dz).
    pub const fn offset(self) -> (i32, i32, i32) {
        match self {
            Self::Top => (0, 1, 0),
            Self::Bottom => (0, -1, 0),
            Self::North => (0, 0, -1),
            Self::South => (0, 0, 1),
            Self::West => (-1, 0, 0),
            Self::East => (1, 0, 0),
        }
    }
}

/// Meshes a single `Chunk` taking adjacent blocks from `World` into account
/// to cull faces between solid blocks and across chunk boundaries.
pub fn mesh_chunk(chunk: &Chunk, world: &World) -> ChunkMesh {
    let mut mesh = ChunkMesh::new();

    let chunk_world_x = chunk.x * 16;
    let chunk_world_z = chunk.z * 16;

    for (section_idx, section_opt) in chunk.sections.iter().enumerate() {
        let section = match section_opt {
            Some(s) => s,
            None => continue,
        };

        let section_y = section_idx * 16;

        for local_y in 0..16 {
            let world_y = (section_y + local_y) as i32;

            for local_z in 0..16 {
                let world_z = chunk_world_z + local_z as i32;

                for local_x in 0..16 {
                    let world_x = chunk_world_x + local_x as i32;
                    let block = section.get_block(local_x, local_y, local_z);

                    if block.is_air() {
                        continue;
                    }

                    // Check all 6 faces for face culling
                    let faces = [
                        BlockFace::Top,
                        BlockFace::Bottom,
                        BlockFace::North,
                        BlockFace::South,
                        BlockFace::West,
                        BlockFace::East,
                    ];

                    for face in faces {
                        let (dx, dy, dz) = face.offset();
                        let neighbor_x = world_x + dx;
                        let neighbor_y = world_y + dy;
                        let neighbor_z = world_z + dz;

                        let neighbor_block = world.get_block(neighbor_x, neighbor_y, neighbor_z);

                        // If neighbor is an opaque cube, cull this face!
                        if neighbor_block.is_opaque_cube() {
                            continue;
                        }

                        // Otherwise, generate 4 vertices and 2 triangles for this visible face
                        append_face_quad(
                            &mut mesh,
                            world_x as f32,
                            world_y as f32,
                            world_z as f32,
                            face,
                            block,
                        );
                    }
                }
            }
        }
    }

    mesh
}

/// Meshes all loaded chunks in the `World` into a combined renderable `ChunkMesh`.
pub fn mesh_world(world: &World) -> ChunkMesh {
    let mut combined_mesh = ChunkMesh::new();

    for chunk in world.iter_chunks() {
        let chunk_mesh = mesh_chunk(chunk, world);
        let base_idx = combined_mesh.vertices.len() as u32;

        combined_mesh.vertices.extend(chunk_mesh.vertices);
        combined_mesh
            .indices
            .extend(chunk_mesh.indices.into_iter().map(|idx| idx + base_idx));
    }

    combined_mesh
}

fn append_face_quad(
    mesh: &mut ChunkMesh,
    x: f32,
    y: f32,
    z: f32,
    face: BlockFace,
    block: BlockState,
) {
    let normal = face.normal();
    // Shading multiplier according to face normal
    let light = match face {
        BlockFace::Top => 1.0,
        BlockFace::Bottom => 0.5,
        BlockFace::East | BlockFace::West => 0.8,
        BlockFace::North | BlockFace::South => 0.6,
    };
    let color = block.debug_color();

    let base_idx = mesh.vertices.len() as u32;

    // 4 vertex coordinates based on face
    let (v0, v1, v2, v3) = match face {
        BlockFace::Top => (
            Vec3::new(x, y + 1.0, z + 1.0),
            Vec3::new(x + 1.0, y + 1.0, z + 1.0),
            Vec3::new(x + 1.0, y + 1.0, z),
            Vec3::new(x, y + 1.0, z),
        ),
        BlockFace::Bottom => (
            Vec3::new(x, y, z),
            Vec3::new(x + 1.0, y, z),
            Vec3::new(x + 1.0, y, z + 1.0),
            Vec3::new(x, y, z + 1.0),
        ),
        BlockFace::North => (
            Vec3::new(x + 1.0, y, z),
            Vec3::new(x, y, z),
            Vec3::new(x, y + 1.0, z),
            Vec3::new(x + 1.0, y + 1.0, z),
        ),
        BlockFace::South => (
            Vec3::new(x, y, z + 1.0),
            Vec3::new(x + 1.0, y, z + 1.0),
            Vec3::new(x + 1.0, y + 1.0, z + 1.0),
            Vec3::new(x, y + 1.0, z + 1.0),
        ),
        BlockFace::West => (
            Vec3::new(x, y, z),
            Vec3::new(x, y, z + 1.0),
            Vec3::new(x, y + 1.0, z + 1.0),
            Vec3::new(x, y + 1.0, z),
        ),
        BlockFace::East => (
            Vec3::new(x + 1.0, y, z + 1.0),
            Vec3::new(x + 1.0, y, z),
            Vec3::new(x + 1.0, y + 1.0, z),
            Vec3::new(x + 1.0, y + 1.0, z + 1.0),
        ),
    };

    let uv0 = Vec2::new(0.0, 1.0);
    let uv1 = Vec2::new(1.0, 1.0);
    let uv2 = Vec2::new(1.0, 0.0);
    let uv3 = Vec2::new(0.0, 0.0);

    mesh.vertices
        .push(BlockVertex::new(v0, normal, uv0, light, color));
    mesh.vertices
        .push(BlockVertex::new(v1, normal, uv1, light, color));
    mesh.vertices
        .push(BlockVertex::new(v2, normal, uv2, light, color));
    mesh.vertices
        .push(BlockVertex::new(v3, normal, uv3, light, color));

    // Two triangles for the quad (CCW winding)
    mesh.indices.extend_from_slice(&[
        base_idx,
        base_idx + 1,
        base_idx + 2,
        base_idx,
        base_idx + 2,
        base_idx + 3,
    ]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_isolated_block_has_6_faces_12_triangles() {
        let mut world = World::new();
        let mut chunk = Chunk::new(0, 0);
        let stone = BlockState::new(1, 0);
        chunk.set_block(5, 5, 5, stone);
        world.insert_chunk(chunk.clone());

        let mesh = mesh_chunk(&chunk, &world);

        // 6 faces * 4 vertices = 24 vertices
        assert_eq!(mesh.vertices.len(), 24);
        // 6 faces * 2 triangles * 3 indices = 36 indices
        assert_eq!(mesh.indices.len(), 36);
    }

    #[test]
    fn contiguous_blocks_cull_internal_faces() {
        let mut world = World::new();
        let mut chunk = Chunk::new(0, 0);
        let stone = BlockState::new(1, 0);

        // Two stone blocks side-by-side along X
        chunk.set_block(5, 5, 5, stone);
        chunk.set_block(6, 5, 5, stone);
        world.insert_chunk(chunk.clone());

        let mesh = mesh_chunk(&chunk, &world);

        // 2 cubes = 12 faces, minus 2 occluded internal touching faces = 10 visible faces
        // 10 faces * 4 vertices = 40 vertices
        assert_eq!(mesh.vertices.len(), 40);
        // 10 faces * 6 indices = 60 indices
        assert_eq!(mesh.indices.len(), 60);
    }
}
