# PRD: Minecraft 1.8.9 Client (Rust) - Fase 3: Decodificación y Tracking del Mundo

## Overview
Implementar las estructuras de datos y algoritmos de decodificación de chunks para Minecraft 1.8.9 en `crates/world`. Permite al cliente interpretar los paquetes de streaming de terreno (`S21 Chunk Data`), almacenar secciones de 16x16x16 bloques con sus 4096 estados de bloque (ID y metadata), iluminación (blocklight y skylight) y biomas, así como realizar consultas espaciales en coordenadas globales $(X, Y, Z)$ con soporte correcto para enteros negativos.

## Goals
- Representar los bloques del juego con `BlockState` (ID de 8 bits y metadata de 4 bits).
- Modelar secciones de chunks (`ChunkSection`: 16x16x16) y columnas completas de terreno (`Chunk`: 16x256x16).
- Decodificar los paquetes `S21 Chunk Data` respetando el formato little-endian de Minecraft 1.8.
- Estructurar el gestor espacial `World` con consultas rápidas euclidianas para bloques en el mapa.
- Integrar la recepción y almacenamiento de chunks en el bucle principal de `crates/client`.

## Quality Gates
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo fmt --check`

## User Stories

### US-009: Tipos BlockState y ChunkSection
**Status:** Completado
- [x] Crear `crates/world/src/block.rs` con empaquetado/desempaquetado de 16 bits para (ID << 4 | metadata).
- [x] Determinar opacidad para culling de caras (`is_opaque_cube`).
- [x] Crear `crates/world/src/chunk.rs` con almacenamiento compacto para 4096 bloques e iluminación.

### US-010: Decodificador S21 Chunk Data
**Status:** Completado
- [x] Crear `crates/world/src/decoder.rs` para parsear bitmasks de secciones y datos de terreno.
- [x] Deserializar bloques en formato little-endian `u16_le`.
- [x] Leer arreglos de iluminación y biomas (256 bytes).
- [x] Añadir prueba unitaria de decodificación completa de chunk.

### US-011: Contenedor espacial `World` y enlace con `client`
**Status:** Completado
- [x] Crear `crates/world/src/world.rs` con consultas euclidianas `get_block` y `set_block`.
- [x] Actualizar `crates/client/src/main.rs` para registrar chunks entrantes con ID `0x21`.
