# PRD: Minecraft 1.8.9 Client (Rust) - Fase 5: Física Vanilla y Bucle de Juego a 20 TPS

## Overview
Implementar en `crates/world` y `crates/client` el motor de física vanilla de Minecraft 1.8.9 y el bucle de sincronización de ticks a 20 TPS (50 ms). Incluye las cajas de colisión AABB del jugador (ancho 0.6, alto 1.8), gravedad exacta (0.08 bloques/tick²), arrastre vertical (0.98), fricción sobre bloques sólidos y aire, resolución de colisiones continuas en ejes X, Y y Z contra el terreno y envío recurrente de paquetes de posición `C06 Player Position And Look`.

## Goals
- Definir la estructura geométrica de colisión `Aabb` (caja alineada con los ejes) en `crates/world`.
- Implementar la resolución de colisiones y cálculo de offsets por eje (X, Y, Z) para evitar penetración de bloques sólidos.
- Implementar `PhysicsEntity` con aceleración de movimiento orientada por el ángulo Yaw, salto vanilla (`JUMP_VELOCITY = 0.42`) y gravedad.
- Configurar un temporizador periódico asíncrono a 20 TPS (50 ms) que actualice la física e informe la posición al servidor (`C06`).

## Quality Gates
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo fmt --check`

## User Stories

### US-015: Cajas de colisión AABB y resolución contra bloques
**Status:** Completado
- [x] Crear `crates/world/src/aabb.rs` con cálculo de intersección y penetración (`calculate_x_offset`, `calculate_y_offset`, `calculate_z_offset`).
- [x] Dimensionar la caja del jugador: ancho 0.6 bloques (radio 0.3) y alto 1.8 bloques.
- [x] Añadir pruebas unitarias de intersección y dimensiones.

### US-016: Motor de Física Vanilla (20 TPS)
**Status:** Completado
- [x] Crear `crates/client/src/physics.rs` con constantes canónicas de Minecraft 1.8.9 (`GRAVITY = 0.08`, `VERTICAL_DRAG = 0.98`, `GROUND_HORIZONTAL_DRAG = 0.546`, `JUMP_VELOCITY = 0.42`).
- [x] Resolver movimiento en orden: Y (gravedad y soporte sobre suelo), X y Z contra bloques circundantes en `World`.
- [x] Pruebas unitarias de caída libre por gravedad y aterrizaje sólido en el suelo.

### US-017: Bucle de Ticks y reporte de posición periódica
**Status:** Completado
- [x] Orquestar un bucle en segundo plano con `tokio::time::interval(50ms)` en `crates/client/src/main.rs`.
- [x] Enviar paquetes `C06 Player Position And Look` a 20 Hz manteniendo al servidor sincronizado con la posición real del jugador.
