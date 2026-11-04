//! Minecraft 1.8.9 Clean-Room Client binary entry point.

pub mod embedded_server;
pub mod physics;

use embedded_server::EmbeddedServer;
use glam::{DVec3, Vec3};
use minecraft_protocol::{
    ClientPlayerPositionAndLookPacket, JoinGamePacket, NetworkConnection, NetworkEvent,
    PlayerPositionAndLookPacket,
};
use minecraft_renderer::{mesh_world, Camera, ChunkMesh, RenderContext};
use minecraft_world::{decode_chunk_data, World};
use physics::PhysicsEntity;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, ElementState, Event, KeyEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::WindowBuilder;

fn main() -> anyhow::Result<()> {
    // Setup structured logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    info!("Starting Minecraft Java 1.8.9 Clean-Room Client (Protocol 47)...");

    // Start background Tokio multi-thread runtime for network and physics
    let rt = tokio::runtime::Runtime::new()?;

    let (event_tx, mut event_rx) = mpsc::channel(100);
    let (conn_tx, conn_rx) = tokio::sync::oneshot::channel::<Arc<NetworkConnection>>();

    // 1. Networking background task
    rt.spawn(async move {
        let default_server: SocketAddr = "127.0.0.1:25565".parse().unwrap();
        let username = "RustPlayer";

        let target_addr = match TcpStream::connect(default_server).await {
            Ok(_) => {
                info!("External Minecraft server detected on {default_server}.");
                default_server
            }
            Err(_) => {
                info!("No external server detected. Launching integrated offline server automatically...");
                let server = EmbeddedServer::start().await.expect("Failed to start embedded server");
                server.addr()
            }
        };

        match NetworkConnection::connect(target_addr, username, event_tx).await {
            Ok(conn) => {
                let conn = Arc::new(conn);
                let _ = conn_tx.send(conn);
            }
            Err(e) => {
                warn!("Connection failed: {e}");
            }
        }
    });

    let connection: Arc<NetworkConnection> = rt.block_on(conn_rx)?;
    info!("Network connection established successfully!");

    let world = Arc::new(Mutex::new(World::new()));
    let player = Arc::new(Mutex::new(PhysicsEntity::new(DVec3::new(0.5, 65.0, 0.5))));
    // Sensitivity constant for FPS mouse look (degrees per pixel of raw motion).
    // Exposed here so it can be tuned without hunting for magic numbers.
    const MOUSE_SENSITIVITY: f32 = 0.15;
    let pressed_keys = Arc::new(Mutex::new(HashSet::<KeyCode>::new()));

    // 2. Physics & Position reporting background task (20 TPS / 50ms)
    let world_phys = Arc::clone(&world);
    let player_phys = Arc::clone(&player);
    let conn_phys = Arc::clone(&connection);
    let keys_phys = Arc::clone(&pressed_keys);

    rt.spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(50));
        loop {
            interval.tick().await;

            let (forward, strafe, jump) = {
                let keys = keys_phys.lock().unwrap();
                let mut f = 0.0;
                let mut s = 0.0;
                if keys.contains(&KeyCode::KeyW) {
                    f += 1.0;
                }
                if keys.contains(&KeyCode::KeyS) {
                    f -= 1.0;
                }
                if keys.contains(&KeyCode::KeyA) {
                    s -= 1.0;
                }
                if keys.contains(&KeyCode::KeyD) {
                    s += 1.0;
                }
                let j = keys.contains(&KeyCode::Space);
                (f, s, j)
            };

            let (pos, yaw, pitch, on_ground) = {
                let w = world_phys.lock().unwrap();
                let mut p = player_phys.lock().unwrap();
                p.tick(&w, forward, strafe, jump);
                (p.position, p.yaw, p.pitch, p.on_ground)
            };

            let pkt =
                ClientPlayerPositionAndLookPacket::new(pos.x, pos.y, pos.z, yaw, pitch, on_ground);
            if conn_phys.send_packet(pkt.to_raw_packet()).await.is_err() {
                break;
            }
        }
    });

    // 3. Network Events listener task
    let world_net = Arc::clone(&world);
    let player_net = Arc::clone(&player);
    let conn_net = Arc::clone(&connection);
    let mesh_dirty = Arc::new(Mutex::new(true));
    let mesh_dirty_net = Arc::clone(&mesh_dirty);

    rt.spawn(async move {
        while let Some(event) = event_rx.recv().await {
            match event {
                NetworkEvent::PacketReceived(pkt) => match pkt.id {
                    0x01 => {
                        if let Ok(join) = JoinGamePacket::from_payload(pkt.payload) {
                            info!("Joined Game! Entity ID: {}", join.entity_id);
                        }
                    }
                    0x08 => {
                        if let Ok(pos) = PlayerPositionAndLookPacket::from_payload(pkt.payload) {
                            {
                                let mut p = player_net.lock().unwrap();
                                p.position = DVec3::new(pos.x, pos.y, pos.z);
                                p.yaw = pos.yaw;
                                p.pitch = pos.pitch;
                            }
                            let ack = ClientPlayerPositionAndLookPacket::new(
                                pos.x, pos.y, pos.z, pos.yaw, pos.pitch, true,
                            );
                            let _ = conn_net.send_packet(ack.to_raw_packet()).await;
                        }
                    }
                    0x21 => {
                        let mut payload = pkt.payload;
                        if let Ok(chunk) = decode_chunk_data(&mut payload) {
                            let mut w = world_net.lock().unwrap();
                            w.insert_chunk(chunk);
                            *mesh_dirty_net.lock().unwrap() = true;
                            info!("Active chunks loaded: {}", w.loaded_chunk_count());
                        }
                    }
                    _ => {}
                },
                NetworkEvent::Disconnected(reason) => {
                    warn!("Disconnected: {reason}");
                    break;
                }
                _ => {}
            }
        }
    });

    // 4. GUI Window and 3D Render Loop with Winit and WGPU
    let event_loop = EventLoop::new()?;
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Minecraft 1.8.9 (Rust Clean-Room Client)")
            .with_inner_size(PhysicalSize::new(1280, 720))
            .with_visible(true)
            .build(&event_loop)?,
    );

    let mut render_ctx = rt.block_on(RenderContext::new(window.clone()))?;
    let mut camera = Camera::new(Vec3::new(0.5, 66.62, 0.5), 1280.0 / 720.0);
    let mut current_mesh = ChunkMesh::new();

    // Lock cursor for FPS camera controls; track state to allow toggle with Escape.
    let _ = window.set_cursor_grab(winit::window::CursorGrabMode::Confined);
    window.set_cursor_visible(false);
    // true = cursor captured (FPS mode), false = cursor free (menu/debug mode).
    let mut cursor_grabbed = true;

    // FPS counter for diagnostics — logs to tracing every second.
    let mut fps_frame_count: u32 = 0;
    let mut fps_timer = Instant::now();

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);

        match event {
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta: (dx, dy) },
                ..
            } => {
                // Only rotate when cursor is captured (FPS mode active).
                if cursor_grabbed {
                    let mut p = player.lock().unwrap();
                    p.yaw += dx as f32 * MOUSE_SENSITIVITY;
                    p.pitch = (p.pitch - dy as f32 * MOUSE_SENSITIVITY).clamp(-89.9, 89.9);
                }
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::Resized(size) => {
                    render_ctx.resize(size);
                    camera.aspect = size.width as f32 / size.height.max(1) as f32;
                }
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: PhysicalKey::Code(key_code),
                            state,
                            ..
                        },
                    ..
                } => {
                    let mut keys = pressed_keys.lock().unwrap();
                    if state == ElementState::Pressed {
                        if key_code == KeyCode::Escape {
                            // Toggle cursor capture: Escape releases/re-captures the cursor.
                            // This matches vanilla Minecraft behaviour (Esc opens the pause menu
                            // and releases mouse; clicking the window re-captures it).
                            cursor_grabbed = !cursor_grabbed;
                            if cursor_grabbed {
                                let _ = window
                                    .set_cursor_grab(winit::window::CursorGrabMode::Confined);
                                window.set_cursor_visible(false);
                            } else {
                                let _ =
                                    window.set_cursor_grab(winit::window::CursorGrabMode::None);
                                window.set_cursor_visible(true);
                            }
                        }
                        keys.insert(key_code);
                    } else {
                        keys.remove(&key_code);
                    }
                }
                WindowEvent::RedrawRequested => {
                    // Update camera from player state
                    {
                        let p = player.lock().unwrap();
                        camera.position = Vec3::new(
                            p.position.x as f32,
                            (p.position.y + 1.62) as f32,
                            p.position.z as f32,
                        );
                        camera.yaw = p.yaw - 90.0;
                        camera.pitch = p.pitch;
                    }
                    render_ctx.update_camera(&camera);

                    // Rebuild chunk mesh if new chunks arrived
                    let mut dirty = mesh_dirty.lock().unwrap();
                    if *dirty {
                        let w = world.lock().unwrap();
                        current_mesh = mesh_world(&w);
                        *dirty = false;
                    }

                    if let Err(e) = render_ctx.render(&current_mesh) {
                        match e {
                            wgpu::SurfaceError::Lost => render_ctx.resize(render_ctx.size),
                            wgpu::SurfaceError::OutOfMemory => elwt.exit(),
                            _ => {}
                        }
                    }

                    // FPS diagnostic counter: logs the measured frame rate every second.
                    fps_frame_count += 1;
                    let elapsed = fps_timer.elapsed();
                    if elapsed >= Duration::from_secs(1) {
                        let fps = fps_frame_count as f64 / elapsed.as_secs_f64();
                        info!("FPS: {fps:.0}");
                        fps_frame_count = 0;
                        fps_timer = Instant::now();
                    }
                }
                _ => {}
            },
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => {}
        }
    })?;

    Ok(())
}
