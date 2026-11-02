# Directrices para Agentes de IA (AGENTS.md)

Este documento contiene las reglas, decisiones de arquitectura y convenciones de codificación obligatorias para cualquier agente de IA que trabaje en este repositorio.

---

## 1. Misión y Alcance del Proyecto

* **Misión:** Desarrollar un cliente compatible con **Minecraft Java Edition 1.8.9 (Protocolo 47)** completamente implementado en **Rust**, utilizando una arquitectura modular (*Cargo Workspace*).
* **Alcance Actual:** Exclusivamente el **Cliente** (red TCP, decodificación de paquetes, tracking del mundo, renderizado con `wgpu`, física básica y control de cámara).

### Reglas Éticas y Legales de Ingeniería Inversa (Clean-Room)
* **ESTRICTO:** No copies ni pegues código decompilado con derechos de autor de Mojang/Microsoft.
* Toda la lógica debe implementarse desde cero utilizando las especificaciones de protocolo abiertas (`Protocol 47` en `wiki.vg` y `mc-packet-reference`).
* Los assets (texturas, modelos, sonidos) nunca deben incluirse en el repositorio. Deben cargarse desde el `.jar` local del usuario en tiempo de ejecución o utilizar recursos de dominio público/libres.

---

## 2. Convenciones de Código en Rust

Sigue siempre las directrices de [`rust-best-practices`](https://github.com/apollographql/rust-best-practices):

1. **Gestión de Errores:**
   * Usa [`thiserror`](https://docs.rs/thiserror) para definir tipos de error estructurados dentro de las librerías (`crates/protocol`, `crates/world`, `crates/renderer`).
   * No uses `.unwrap()` ni `.expect()` en rutas de ejecución en producción. Usa propagación de errores con el operador `?`.
2. **Propiedad y Préstamo (Borrowing vs Cloning):**
   * Prefiere pasar referencias prestadas (`&[u8]`, `&str`) en lugar de clonar (`.clone()`).
   * Utiliza buffers reutilizables (`bytes::BytesMut`, `Cursor<&[u8]>`) para procesar el flujo TCP sin sobrecargar el asignador de memoria (evitar allocations innecesarias en el loop de red y renderizado).
3. **Calidad y Linter:**
   * Ejecuta siempre: `cargo clippy --all-targets --all-features -- -D warnings`
   * Formatea con: `cargo fmt`
4. **Comentarios y Documentación:**
   * Usa comentarios `//` para explicar el *por qué* de decisiones técnicas o trucos a nivel de bits.
   * Documenta funciones públicas con `///` indicando precondiciones y comportamiento esperado.

---

## 3. Estructura del Cargo Workspace

Mantén la separación estricta de responsabilidades entre crates:

* [`crates/protocol`](crates/protocol):
  * Primitivas de Minecraft 1.8 (`VarInt`, `VarLong`, `Position`, cadenas UTF-8, etc.).
  * Máquina de estados (`Handshake`, `Status`, `Login`, `Play`).
  * Encriptación (AES-128 CFB8) y descompresión Zlib (`flate2`).
  * Codificación y decodificación de paquetes de red.
* [`crates/world`](crates/world):
  * Almacenamiento en memoria de chunks (secciones de 16×16×16 bloques).
  * Decodificación de paquetes de chunk (`S21 Chunk Data` y `S26 Map Chunk Bulk`).
  * Consultas rápidas de bloques por coordenadas globales `(x, y, z)`.
* [`crates/renderer`](crates/renderer):
  * Pipeline gráfico basado en `wgpu` y `winit`.
  * Shaders en WGSL.
  * *Face culling mesher*: Generación eficiente de mallas poligonales descartando caras ocluidas.
  * Matrices de cámara (View / Projection / Frustum).
* [`crates/client`](crates/client):
  * Bucle de juego principal: sincronización de ticks de física a 20 TPS y renderizado a vsync/60+ FPS.
  * Entrada del usuario (WASD, ratón, saltos).
  * Orquestación de la conexión con el servidor.

---

## 4. Uso de la Skill REA (Reverse Engineer Anything)

Si durante el desarrollo se detecta una discrepancia en el comportamiento del cliente frente al juego original (por ejemplo: desalineación en el cálculo de colisiones AABB, orden exacto de bytes en un paquete no documentado, o formato de modelos JSON de la 1.8):
1. **Inspección de artefactos:** Usa `open_binary` e `inspect_artifact` con el archivo `1.8.jar` para verificar la estructura de recursos internos.
2. **Análisis de capturas de red:** Usa `inspect_packet_capture` con un archivo `.pcap` si necesitas validar el flujo de paquetes exacto entre un cliente y servidor 1.8 vanilla.
3. **Registro de hallazgos:** Anota las evidencias en la documentación antes de modificar la implementación.

---

## 5. Tarea Inmediata / Próximo Paso Recomendado

1. Inicializar el workspace de Cargo:
   * Crear el [`Cargo.toml`](Cargo.toml) raíz con los miembros del workspace.
   * Inicializar el crate [`crates/protocol`](crates/protocol).
2. Implementar los tipos primitivos del Protocolo 47:
   * `VarInt` y `VarLong` con métodos `read_varint` y `write_varint`.
   * `Position` (desempaquetado de coordenadas `x`, `y`, `z` a partir de un entero de 64 bits).
   * Pruebas unitarias para validar la codificación y decodificación exacta.
