//! Vertex definitions and meshing data structures for chunk rendering.

use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3};

/// A single 3D vertex for voxel rendering.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default, Pod, Zeroable)]
pub struct BlockVertex {
    /// 3D position in world or chunk coordinates.
    pub position: [f32; 3],
    /// Normal vector for basic diffuse shading.
    pub normal: [f32; 3],
    /// Texture UV coordinates.
    pub uv: [f32; 2],
    /// Vertex AO or lighting factor (0.0 to 1.0).
    pub light: f32,
}

impl BlockVertex {
    /// Vertex buffer layout for WGPU pipeline.
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<BlockVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position: vec3<f32>
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // normal: vec3<f32>
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // uv: vec2<f32>
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() * 2) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // light: f32
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() * 2 + std::mem::size_of::<[f32; 2]>())
                        as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }

    /// Creates a new `BlockVertex`.
    pub const fn new(position: Vec3, normal: Vec3, uv: Vec2, light: f32) -> Self {
        Self {
            position: [position.x, position.y, position.z],
            normal: [normal.x, normal.y, normal.z],
            uv: [uv.x, uv.y],
            light,
        }
    }
}

/// Generated polygonal mesh for a chunk or chunk section.
#[derive(Debug, Clone, Default)]
pub struct ChunkMesh {
    /// List of vertices.
    pub vertices: Vec<BlockVertex>,
    /// Index buffer for triangle rendering (3 indices per triangle).
    pub indices: Vec<u32>,
}

impl ChunkMesh {
    /// Creates an empty chunk mesh.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if the mesh contains no triangles.
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// Clears the mesh buffers.
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }
}
