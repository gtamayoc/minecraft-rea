# PRD: Minecraft 1.8.9 Client (Rust) - Fase 4: Culled Face Mesher y Cámara 3D

## Overview
Implementar en `crates/renderer` el motor de generación geométrica para vóxeles (*Culled Face Meshing*) y la cámara en primera persona con matrices de transformación View y Projection en 3D. El mesher descarta de forma óptima las caras ocluidas entre bloques sólidos adyacentes tanto dentro de la misma sección como a través de las fronteras entre chunks contiguos, reduciendo drásticamente la cantidad de polígonos a dibujar en la GPU.

## Goals
- Definir la estructura de vértices de bloques `BlockVertex` con posición 3D, normales, coordenadas UV y factor de iluminación/sombreado por cara.
- Implementar el algoritmo de *Culled Face Meshing* para chunks (`mesh_chunk`).
- Implementar la cámara 3D en primera persona con rotación Yaw/Pitch (sujeción a [-89.9°, 89.9°]) y cálculo de matrices `view_matrix` y `projection_matrix` con Glam.
- Proveer los shaders WGSL para renderizado de bloques (`vs_main` y `fs_main`).

## Quality Gates
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo fmt --check`

## User Stories

### US-012: Vértices y buffers de malla
**Status:** Completado
- [x] Crear `crates/renderer/src/vertex.rs` definiendo `BlockVertex` y `ChunkMesh` (vértices e índices de triángulos).

### US-013: Algoritmo Culled Face Mesher
**Status:** Completado
- [x] Crear `crates/renderer/src/mesher.rs` con soporte para las 6 caras (`Top`, `Bottom`, `North`, `South`, `West`, `East`).
- [x] Descartar caras cuando el bloque contiguo en `World` es un cubo opaco (`is_opaque_cube`).
- [x] Calcular iluminación direccional según la orientación de la cara.
- [x] Añadir pruebas unitarias verificando que 2 cubos pegados descartan las 2 caras de contacto internas.

### US-014: Cámara en Primera Persona y Shaders WGSL
**Status:** Completado
- [x] Crear `crates/renderer/src/camera.rs` con vectores `forward`, `right`, `up` y matrices View/Projection usando `glam`.
- [x] Crear `crates/renderer/src/shader.rs` con el pipeline de shaders WGSL para la GPU.
- [x] Añadir pruebas unitarias de orientación y cálculo de matriz combinada View-Projection.
