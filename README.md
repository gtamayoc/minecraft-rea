# Rust Minecraft 1.8 Client (Clean-Room Reimplementation)

Implementación limpia (*clean-room*) de un cliente para **Minecraft Java Edition 1.8 / 1.8.9 (Protocolo 47)** escrito desde cero en **Rust**.

---

## 1. Visión y Objetivos

* **Objetivo:** Desarrollar un cliente ligero, rápido y moderno en Rust capaz de conectarse a cualquier servidor Minecraft 1.8.x estándar (Vanilla, Spigot, Paper).
* **Enfoque Clean-Room:**
  * **Sin código descompilado copiado:** Todo el código se escribe desde cero basándose en especificaciones abiertas de protocolo y formatos de datos.
  * **Sin assets propietarios distribuidos:** Texturas y sonidos se cargan en tiempo de ejecución desde el `.jar` oficial del usuario o desde texture packs de código abierto/libres.
* **Versión de destino:** Minecraft 1.8.9 (`protocol_version = 47`).

---

## 2. Pila Tecnológica (Tech Stack)

* **Lenguaje:** Rust 2021+ (`rustc 1.95+`).
* **Gráficos y Ventana:** [`wgpu`](https://github.com/gfx-rs/wgpu) (DirectX 12 / Vulkan / Metal) + [`winit`](https://github.com/rust-windowing/winit).
* **Matemáticas 3D:** [`glam`](https://github.com/bitshifter/glam-rs) (SIMD-accelerated linear algebra).
* **Red y Networking:** [`tokio`](https://tokio.rs/) o hilos dedicados con `std::net::TcpStream` + [`bytes`](https://github.com/tokio-rs/bytes).
* **Manejo de Errores:** [`thiserror`](https://github.com/dtolnay/thiserror) para librerías y crates modulares.
* **Compresión:** [`flate2`](https://github.com/rust-lang/flate2-rs) (zlib / deflate para compresión de paquetes y chunks).
* **Parsing NBT:** `fastnbt` o `simdnbt`.

---

## 3. Arquitectura del Workspace Cargo

El proyecto está diseñado como un **Cargo Workspace** modular:

```text
minecraft-rea/
├── Cargo.toml                  # Workspace raíz
├── README.md                   # Documentación general del proyecto
├── AGENTS.md                   # Instrucciones específicas para agentes de IA
└── crates/
    ├── protocol/               # Protocolo 47: VarInt, frames, paquetes, codecs
    ├── world/                  # Chunks 1.8 (16x16x16 subchunks), paletas, bloques
    ├── renderer/               # Renderizado WGPU: chunk meshing (face culling), cámara, shaders
    └── client/                 # Binario ejecutable: game loop, input, física y orquestación
```

### Detalle de Crates

1. **`crates/protocol`**:
   * Codificación y decodificación de tipos básicos: `VarInt`, `VarLong`, `Position` (u64 bit-packed), `McString`, `Slot`.
   * Máquina de estados: `Handshaking`, `Status`, `Login`, `Play`.
   * Compresión adaptativa (cuando el servidor envía `Set Compression`).
   * Manejo de cifrado AES-128 CFB8 (para servidores en línea autenticados).
2. **`crates/world`**:
   * Estructura de almacenamiento de chunks en memoria (`(ChunkX, ChunkZ) -> Chunk`).
   * Formato 1.8: Cada chunk vertical tiene hasta 16 secciones (16×16×16 bloques). Cada bloque tiene ID (8 bits) y Metadata (4 bits).
   * Lector de paquetes `S21 Chunk Data` y `S26 Map Chunk Bulk`.
3. **`crates/renderer`**:
   * Inicialización de superficie y adaptador con `wgpu`.
   * Algoritmo de *Culled Face Meshing* (no generar polígonos entre bloques sólidos contiguos).
   * Cámara en primera persona (cálculo de matrices View y Projection, Pitch y Yaw).
   * Pipeline de renderizado con shaders WGSL.
4. **`crates/client`**:
   * Bucle de juego principal a 60+ FPS y tick de física a 20 TPS (50 ms por tick).
   * Entrada de teclado/ratón (`winit`), reporte de posición (`C04`/`C06`), KeepAlive (`C00`).

---

## 4. Referencia del Protocolo 47 (Minecraft 1.8.x)

### 4.1 Tipos de Datos Primitivos
* **`VarInt`**: Entero de 32 bits de longitud variable (LEB128 de 7 bits, 1 a 5 bytes). El bit más significativo (MSB) indica si siguen más bytes.
* **`VarLong`**: Entero de 64 bits de longitud variable (1 a 10 bytes).
* **`String`**: Cadena UTF-8 precedida por un `VarInt` que indica su longitud en bytes (máximo 32767 caracteres).
* **`Position`**: Coordenadas de bloque empaquetadas en un entero sin signo de 64 bits (`u64`):
  * $X$: 26 bits con signo (`val >> 38`).
  * $Y$: 12 bits sin signo (`(val >> 26) & 0xFFF`).
  * $Z$: 26 bits con signo (`val << 38 >> 38`).

### 4.2 Secuencia de Conexión (Handshake & Login)
1. **Cliente envía `Handshake` (`0x00`)**:
   * `protocol_version`: 47 (VarInt)
   * `server_address`: String (ej. "localhost")
   * `server_port`: u16 (ej. 25565)
   * `next_state`: 2 (VarInt para Login, o 1 para Status ping)
2. **Cliente envía `Login Start` (`0x00`)**:
   * `player_name`: String (ej. "RustPlayer")
3. **Servidor responde con**:
   * `Set Compression` (`0x03`, opcional): Umbral de compresión `VarInt`.
   * `Login Success` (`0x02`): UUID del jugador (String) y nombre de usuario.
4. **Transición automática al estado `Play`**:
   * Servidor envía `Join Game` (`0x01`): Entity ID, Gamemode, Dimension, Difficulty, World Type.
   * Servidor envía `Spawn Position` (`0x05`) y `Player Position & Look` (`0x08`).
   * Servidor comienza el streaming de chunks (`S21 Chunk Data`).

---

## 5. Hoja de Ruta y Fases

* [ ] **Fase 1: Protocolo Base y Handshake**
  * Implementar primitivas (`VarInt`, `VarLong`, `Position`, buffers de lectura/escritura).
  * Implementar paquetes de `Handshake`, `Login Start`, `Login Success` y `Set Compression`.
  * Test unitario de serialización y cliente TCP de prueba que complete el handshake contra un servidor local.
* [ ] **Fase 2: Estado Play y Supervivencia de Conexión**
  * Responder automáticamente a `S00 Keep Alive` con `C00 Keep Alive`.
  * Procesar `S01 Join Game` y registrar el Entity ID y dimensiones.
  * Procesar `S08 Player Position & Look` y responder con confirmación de posición.
* [ ] **Fase 3: Decodificación del Mundo**
  * Decodificar `S21 Chunk Data` y `S26 Map Chunk Bulk` (descompresión Zlib y parsing de bloques).
  * Crear la estructura `World` con mapa de chunks cargados.
* [ ] **Fase 4: Ventana y Renderizado WGPU**
  * Configurar ventana con `winit` y contexto gráfico con `wgpu`.
  * Shaders básicos WGSL e iluminación simple.
  * Generación de malla de bloques con descarte de caras internas (*face culling*).
  * Cámara en primera persona navegable con ratón y teclado.
* [ ] **Fase 5: Física y Bucle de Juego**
  * Tick de física a 20 TPS (50 ms) con gravedad vanilla (0.08 bloques/tick² y fricción).
  * Colisiones AABB jugador vs bloques.
  * Reporte periódico de posición (`C04 Player Position` / `C06 Player Position And Look`).

---

## 6. Recursos y Enlaces Útiles

* [Especificación Protocol 47 (mc-packet-reference)](https://mc-packet-reference.netlify.app/1.8.9/packets/)
* [Wiki.vg Protocol Documentation](https://wiki.vg/Protocol)
* [Minecraft-Data (PrismarineJS) - 1.8.8](https://github.com/PrismarineJS/minecraft-data)
* [CrabCraft (Referencia de arquitectura WGPU en Rust)](https://github.com/Solenopsisbot/CrabCraft)
