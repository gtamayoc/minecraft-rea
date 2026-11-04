# PRD: Control de Movimiento, Cámara FPS y Renderizado a 120+ FPS

## Overview
Optimizar la experiencia del jugador en `crates/client` y `crates/renderer` corrigiendo el vector de traslación adelante/atrás alineado con la orientación de la cámara (Yaw/Pitch), implementando captura de mouse en primera persona con sensibilidad configurable y liberando el límite de tasa de refresco a un mínimo de 120 FPS mediante modos de presentación desacoplados (`PresentMode::AutoNoVsync` / `Immediate`) junto a optimizaciones en la preparación de frames.

## Goals
- Corregir el cálculo trigonométrico de desplazamiento adelante (`W`) y atrás (`S`) relativo al ángulo de visión (Yaw) de la cámara.
- Añadir control de cámara estilo FPS con captura y ocultación de cursor del ratón (`DeviceEvent::MouseMotion`), delimitando Pitch entre -89° y +89°.
- Configurar el swapchain de `wgpu` con modo de presentación sin bloqueo vertical obligatorio (`AutoNoVsync` / `Immediate`) para permitir 120+ FPS estables.
- Reducir sobrecostes de CPU/GPU en el render loop eliminando clonaciones y reasignaciones redundantes de memoria durante el dibujo de chunks.
- Mantener un conjunto de Quality Gates rápido y ligero que se ejecute al finalizar todas las historias.

## Quality Gates

These commands must pass at the end of the feature execution:
- `cargo check --workspace`
- `cargo test --workspace`

## User Stories

### US-018: Corrección de dirección de traslación adelante y atrás
**Description:** Como jugador, quiero moverme hacia adelante y hacia atrás en la dirección exacta hacia la que mira la cámara para que los controles respondan de manera natural e intuitiva.

**Acceptance Criteria:**
- [ ] Revisar y corregir en `crates/client/src/physics.rs` o controlador de entrada el cálculo de vectores según el ángulo `yaw` actual.
- [ ] Presionar `W` desplaza al jugador en dirección frontal horizontal `(sin(yaw), -cos(yaw))` o convención canónica del protocolo.
- [ ] Presionar `S` desplaza en sentido opuesto directo sin desviación angular.
- [ ] Añadir prueba unitaria verificando el vector unitario resultante para orientaciones cardinales (Norte, Sur, Este, Oeste).

### US-019: Control de cámara en primera persona con mouse
**Description:** Como jugador, quiero controlar la orientación de la vista moviendo el ratón libremente en modo primera persona con cursor capturado.

**Acceptance Criteria:**
- [ ] Configurar captura de cursor (`CursorGrabMode::Confined` o `Locked`) y visibilidad (`set_cursor_visible(false)`) en la ventana de `winit`.
- [ ] Manejar `winit::event::DeviceEvent::MouseMotion` aplicando sensibilidad para actualizar `yaw` y `pitch`.
- [ ] Restringir el ángulo `pitch` entre -89.0° y +89.0° para prevenir inversión de la matriz de vista.
- [ ] Sincronizar la orientación de la cámara en `crates/renderer` con la entidad del jugador en `crates/client`.

### US-020: Configuración de renderizado de alta tasa de refresco (120+ FPS)
**Description:** Como jugador, quiero que el motor gráfico renderice a un mínimo de 120 FPS sin quedar limitado artificialmente por VSync a 60 Hz.

**Acceptance Criteria:**
- [ ] Configurar `wgpu::SurfaceConfiguration` con `PresentMode::AutoNoVsync` (o fallback a `Immediate` / `Mailbox` según capacidades de la GPU).
- [ ] Eliminar clonaciones y reallocations redundantes en el bucle principal de renderizado (`crates/renderer`).
- [ ] Implementar un contador/medidor de FPS en el bucle del cliente para diagnóstico y verificación de tasa de cuadros.

## Functional Requirements
- FR-1: El desplazamiento relativo del jugador debe proyectarse sobre el plano horizontal XZ tomando en cuenta el ángulo `yaw` de la vista.
- FR-2: El cursor del ratón debe bloquearse al hacer foco en la ventana del cliente y liberarse al presionar la tecla `Escape`.
- FR-3: El movimiento del mouse delta horizontal (`dx`) debe modificar el `yaw` y el delta vertical (`dy`) debe modificar el `pitch`.
- FR-4: El pipeline de renderizado de `wgpu` no debe forzar `PresentMode::Fifo` (VSync bloqueante), garantizando la posibilidad de alcanzar 120+ FPS en hardware compatible.

## Non-Goals
- Soporte para vista en tercera persona (F5).
- Menú gráfico interactivo en pantalla para cambiar resolución o modo de pantalla completa.
- Física avanzada de vuelo o modo espectador.

## Technical Considerations
- `winit` puede requerir alternar entre `CursorGrabMode::Locked` y `CursorGrabMode::Confined` según la plataforma de Windows.
- Los ticks de física se mantienen sincronizados a 20 TPS (50 ms) mediante el loop asíncrono existente, mientras que el renderizado de fotogramas corre desacoplado en el loop principal de la ventana para superar 120 FPS.

## Success Metrics
- Movimiento hacia adelante y atrás 100% alineado con la cruz de mira en cualquier ángulo.
- La rotación de cámara es fluida, sin saltos ni tirones por aceleración errónea.
- La tasa de cuadros por segundo supera consistentemente los 120 FPS en escenarios estándar de renderizado de chunks.
