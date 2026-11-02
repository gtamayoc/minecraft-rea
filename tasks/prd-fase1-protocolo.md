# PRD: Minecraft 1.8.9 Client (Rust) - Fase 1: Workspace & Protocolo Base

## Overview
Crear la base del cliente de juego Minecraft 1.8.9 (Protocolo 47) en Rust. Esta fase inicial establece la arquitectura modular del proyecto (*Cargo Workspace*) e implementa el crate fundamental de comunicación (`crates/protocol`), permitiendo codificar y decodificar los tipos de datos del protocolo (VarInt, VarLong, Position, Strings) y procesar los paquetes iniciales de conexión (Handshake y Login).

## Goals
- Establecer un workspace modular y escalable en Rust para todo el juego (`protocol`, `world`, `renderer`, `client`).
- Implementar los tipos primitivos y paquetes de red de Minecraft 1.8.9 con precisión a nivel de bits (especificación Protocolo 47).
- Asegurar que el código sea seguro, sin pánicos en tiempo de ejecución, con manejo de errores idiomático (`thiserror`).
- Proveer cobertura de pruebas automatizadas para garantizar que cada mensaje y tipo de dato coincida exactamente con el protocolo oficial.

## Quality Gates

These commands must pass for every user story:
- `cargo check --workspace` - Verificación de tipos y compilación limpia
- `cargo test --workspace` - Suite de pruebas unitarias exitosas
- `cargo clippy --all-targets --all-features -- -D warnings` - Linter estricto de Rust sin advertencias
- `cargo fmt --check` - Formato estándar de código

## User Stories

### US-001: Inicializar Cargo Workspace y estructura de directorios
**Status:** Completado
**Description:** Como desarrollador del motor, quiero configurar el archivo raíz `Cargo.toml` y las carpetas de los crates para que el proyecto compile de forma modular y organizada.

**Acceptance Criteria:**
- [x] Crear el archivo raíz `Cargo.toml` definiendo los miembros del workspace: `crates/protocol`, `crates/world`, `crates/renderer`, `crates/client`.
- [x] Configurar el crate de biblioteca `crates/protocol` con su propio `Cargo.toml` y dependencias base (`thiserror`, `bytes`, `tokio`).
- [x] Configurar esqueletos mínimos para `crates/world`, `crates/renderer` y `crates/client` para que el workspace sea válido.
- [x] Ejecutar `cargo check --workspace` sin errores de compilación.

### US-002: Implementar tipos primitivos VarInt y VarLong
**Status:** Completado
**Description:** Como sistema de red del juego, quiero serializar y deserializar enteros de longitud variable (LEB128) para codificar IDs y tamaños de paquetes según la especificación de Minecraft.

**Acceptance Criteria:**
- [x] Crear el módulo `crates/protocol/src/types/varint.rs` con soporte para lectura y escritura de `VarInt` (32 bits, 1 a 5 bytes).
- [x] Implementar soporte para `VarLong` (64 bits, 1 a 10 bytes).
- [x] Retornar errores descriptivos si el número supera el límite de bytes permitido (evitar desbordamiento de buffer o loops infinitos).
- [x] Añadir pruebas unitarias verificando valores clave (ej. 0, 1, 255, 2147483647, -1).

### US-003: Implementar tipo Position y Strings de Minecraft
**Status:** Completado
**Description:** Como sistema del mundo, quiero empaquetar y desempaquetar coordenadas de bloques tridimensionales (X, Y, Z) en enteros de 64 bits y leer cadenas UTF-8 prefijadas por longitud.

**Acceptance Criteria:**
- [x] Crear el módulo `crates/protocol/src/types/position.rs` con la estructura `Position { x: i32, y: i32, z: i32 }`.
- [x] Implementar la codificación y decodificación de 64 bits preservando el signo para coordenadas negativas ($X$: 26 bits, $Y$: 12 bits, $Z$: 26 bits).
- [x] Implementar lectura y escritura de cadenas de texto (String prefijado por `VarInt` con límite de longitud).
- [x] Añadir pruebas unitarias validando coordenadas positivas y negativas, y cadenas con caracteres UTF-8.

### US-004: Implementar Framing de paquetes y máquina de estados
**Status:** Completado
**Description:** Como cliente del juego, quiero encapsular los mensajes en frames con longitud prefijada y gestionar los estados de conexión (Handshaking, Status, Login, Play).

**Acceptance Criteria:**
- [x] Definir el enum `ConnectionState` con variantes `Handshaking`, `Status`, `Login`, `Play`.
- [x] Implementar la lógica de decodificación de frames: leer `packet_length` (VarInt), `packet_id` (VarInt) y el payload de bytes.
- [x] Definir el sistema de errores estructurado en `crates/protocol/src/error.rs` usando `thiserror`.
- [x] Añadir pruebas unitarias simulando un buffer de bytes para validar el correcto corte y ensamblado de frames.

### US-005: Implementar paquetes de Handshake y Login
**Status:** Completado
**Description:** Como jugador que inicia el juego, quiero que el cliente genere los paquetes de presentación para conectarse a un servidor e iniciar sesión.

**Acceptance Criteria:**
- [x] Implementar el paquete saliente `Handshake` (ID `0x00`: protocol version 47, server address, port, next state).
- [x] Implementar el paquete saliente `Login Start` (ID `0x00`: player username).
- [x] Implementar el paquete entrante `Login Success` (ID `0x02`: UUID del jugador y nombre de usuario).
- [x] Implementar el soporte para el paquete entrante `Set Compression` (ID `0x03`: umbral de compresión).
- [x] Añadir pruebas unitarias de serialización/deserialización para cada uno de estos paquetes.

## Functional Requirements
- FR-1: El sistema debe utilizar la especificación oficial del Protocolo 47 (Minecraft 1.8.x).
- FR-2: Toda lectura sobre buffers debe validar la disponibilidad de bytes para evitar pánicos por índice fuera de rango.
- FR-3: La deserialización de enteros negativos en `Position` debe preservar la extensión de signo correcta.
- FR-4: Las estructuras de paquetes deben implementar traits para conversión limpia hacia y desde buffers de bytes.

## Non-Goals
- Renderizado gráfico o apertura de ventana (se abordará en la Fase 4).
- Carga de texturas o lectura de modelos 3D (se abordará en la Fase 4).
- Autenticación online con servidores de Microsoft/Mojang (esta fase se limita a modo offline/directo).
- Movimiento y física del jugador (se abordará en la Fase 5).

## Technical Considerations
- Usar Rust edición 2021+.
- Uso de buffers reutilizables (`bytes::BytesMut`, slices `&[u8]`) para minimizar asignaciones innecesarias de memoria en el procesamiento de paquetes.
- El proyecto debe mantenerse estrictamente bajo metodología clean-room (sin copiar código decompilado).
