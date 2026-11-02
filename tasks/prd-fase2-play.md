# PRD: Minecraft 1.8.9 Client (Rust) - Fase 2: Conexión TCP y Estado Play

## Overview
Implementar la infraestructura de conexión en red asíncrona (`tokio::net::TcpStream`) y los paquetes esenciales del estado `Play` de Minecraft 1.8.9 (Protocolo 47). Esta fase permite al cliente conectarse a un servidor real o local, realizar la secuencia completa de handshake y login, responder automáticamente a pings de Keep-Alive para mantener viva la sesión, recibir información del juego (`Join Game`), posición del spawn, y confirmar la posición del jugador (`Player Position and Look`).

## Goals
- Establecer un gestor de conexión de red bidireccional asíncrono con Tokio.
- Mantener la conexión activa respondiendo automáticamente a paquetes `S00 Keep Alive` con `C00 Keep Alive`.
- Deserializar paquetes de entrada del estado Play: `S01 Join Game`, `S02 Chat Message`, `S05 Spawn Position`, `S08 Player Position And Look`.
- Serializar paquetes de salida del estado Play: `C06 Player Position And Look`.
- Integrar la orquestación en el binario `crates/client` para logging estructurado y conexión en vivo.

## Quality Gates
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo fmt --check`

## User Stories

### US-006: Paquetes del estado Play
**Status:** Completado
- [x] Implementar `ServerKeepAlivePacket` (`0x00`) y `ClientKeepAlivePacket` (`0x00`).
- [x] Implementar `JoinGamePacket` (`0x01`).
- [x] Implementar `ServerChatMessagePacket` (`0x02`).
- [x] Implementar `SpawnPositionPacket` (`0x05`).
- [x] Implementar `PlayerPositionAndLookPacket` (`0x08`) y `ClientPlayerPositionAndLookPacket` (`0x06`).
- [x] Añadir pruebas unitarias de serialización/deserialización para cada paquete.

### US-007: Gestor de conexión de red asíncrono (`NetworkConnection`)
**Status:** Completado
- [x] Crear `crates/protocol/src/connection.rs` utilizando `tokio::net::TcpStream`.
- [x] Implementar bucle de lectura con `FrameDecoder` y buffer reutilizable `BytesMut`.
- [x] Implementar bucle de escritura asíncrono con canales `mpsc`.
- [x] Responder automáticamente a `S00 Keep Alive` para evitar timeout de desconexión.
- [x] Transicionar automáticamente de `Login` a `Play` tras recibir `Login Success` (`0x02`).

### US-008: Conectar `crates/client` con logging estructurado
**Status:** Completado
- [x] Integrar `tracing` y `tracing-subscriber` en `crates/client/src/main.rs`.
- [x] Iniciar conexión a `127.0.0.1:25565` con manejo amigable de errores si el servidor no está encendido.
- [x] Escuchar eventos del servidor y responder a la posición inicial enviando `C06 Player Position And Look`.
