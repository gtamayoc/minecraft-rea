//! Rendering engine based on WGPU for Minecraft 1.8.9.

#![deny(missing_docs)]

pub mod camera;
pub mod context;
pub mod mesher;
pub mod shader;
pub mod vertex;

pub use camera::Camera;
pub use context::RenderContext;
pub use mesher::{mesh_chunk, BlockFace};
pub use shader::CHUNK_SHADER;
pub use vertex::{BlockVertex, ChunkMesh};
