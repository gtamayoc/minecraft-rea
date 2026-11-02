//! Minecraft 1.8.9 Clean-Room Client binary entry point.

pub mod physics;

use glam::DVec3;
use minecraft_protocol::{
    ClientPlayerPositionAndLookPacket, JoinGamePacket, NetworkConnection, NetworkEvent,
    PlayerPositionAndLookPacket, ServerChatMessagePacket, SpawnPositionPacket,
};
use minecraft_world::{decode_chunk_data, World};
use physics::PhysicsEntity;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup structured logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Minecraft Java 1.8.9 Clean-Room Client (Protocol 47)...");

    let server_addr: SocketAddr = "127.0.0.1:25565".parse()?;
    let username = "RustPlayer";

    let (event_tx, mut event_rx) = mpsc::channel(100);

    let connection = match NetworkConnection::connect(server_addr, username, event_tx).await {
        Ok(conn) => Arc::new(conn),
        Err(e) => {
            warn!("Could not connect to {server_addr}: {e}");
            info!("Run a local Minecraft 1.8.9 server with online-mode=false on 127.0.0.1:25565 to test live!");
            return Ok(());
        }
    };

    info!("Connected successfully! Listening for server packets...");
    let world = Arc::new(Mutex::new(World::new()));
    let player = Arc::new(Mutex::new(PhysicsEntity::new(DVec3::new(0.0, 65.0, 0.0))));

    // Start 20 TPS (50 ms) fixed physics and position report loop
    let world_physics = Arc::clone(&world);
    let player_physics = Arc::clone(&player);
    let conn_physics = Arc::clone(&connection);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(50));
        loop {
            interval.tick().await;

            let (pos, yaw, pitch, on_ground) = {
                let w = world_physics.lock().unwrap();
                let mut p = player_physics.lock().unwrap();
                p.tick(&w, 0.0, 0.0, false);
                (p.position, p.yaw, p.pitch, p.on_ground)
            };

            // Report position to server (C06)
            let pkt =
                ClientPlayerPositionAndLookPacket::new(pos.x, pos.y, pos.z, yaw, pitch, on_ground);
            if conn_physics.send_packet(pkt.to_raw_packet()).await.is_err() {
                break;
            }
        }
    });

    while let Some(event) = event_rx.recv().await {
        match event {
            NetworkEvent::StateChanged(state) => {
                info!("Transitioned to state: {:?}", state);
            }
            NetworkEvent::PacketReceived(pkt) => match pkt.id {
                0x01 => {
                    // S01 Join Game
                    if let Ok(join) = JoinGamePacket::from_payload(pkt.payload) {
                        info!(
                            "Joined Game! Entity ID: {}, Gamemode: {}, Dimension: {}, Level Type: {}",
                            join.entity_id, join.gamemode, join.dimension, join.level_type
                        );
                    }
                }
                0x02 => {
                    // S02 Chat Message
                    if let Ok(chat) = ServerChatMessagePacket::from_payload(pkt.payload) {
                        info!("[Server Chat] {}", chat.message_json);
                    }
                }
                0x05 => {
                    // S05 Spawn Position
                    if let Ok(spawn) = SpawnPositionPacket::from_payload(pkt.payload) {
                        info!(
                            "World Spawn Position: ({}, {}, {})",
                            spawn.location.x, spawn.location.y, spawn.location.z
                        );
                    }
                }
                0x08 => {
                    // S08 Player Position And Look
                    if let Ok(pos) = PlayerPositionAndLookPacket::from_payload(pkt.payload) {
                        info!(
                            "Player Initial Position: ({:.2}, {:.2}, {:.2}), Yaw: {:.1}, Pitch: {:.1}",
                            pos.x, pos.y, pos.z, pos.yaw, pos.pitch
                        );

                        {
                            let mut p = player.lock().unwrap();
                            p.position = DVec3::new(pos.x, pos.y, pos.z);
                            p.yaw = pos.yaw;
                            p.pitch = pos.pitch;
                        }

                        // Confirm position back to server (C06)
                        let ack = ClientPlayerPositionAndLookPacket::new(
                            pos.x, pos.y, pos.z, pos.yaw, pos.pitch, true,
                        );
                        let _ = connection.send_packet(ack.to_raw_packet()).await;
                    }
                }
                0x21 => {
                    // S21 Chunk Data
                    let mut payload = pkt.payload;
                    if let Ok(chunk) = decode_chunk_data(&mut payload) {
                        let mut w = world.lock().unwrap();
                        w.insert_chunk(chunk);
                        info!("Total active chunks: {}", w.loaded_chunk_count());
                    }
                }
                _ => {}
            },
            NetworkEvent::Disconnected(reason) => {
                warn!("Disconnected: {reason}");
                break;
            }
        }
    }

    Ok(())
}
