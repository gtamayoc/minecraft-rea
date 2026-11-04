//! Mock/Integrated standalone local server for offline standalone play & testing.
//!
//! Spawns a lightweight local TCP listener simulating a Protocol 47 (Minecraft 1.8.9) server.
//! When no external dedicated server is detected, this server boots automatically in the background,
//! sends JoinGame, SpawnPosition, PlayerPositionAndLook, KeepAlive pings, and streams
//! synthetic terrain chunks so the game can be fully played and tested offline out-of-the-box.

use bytes::{BufMut, BytesMut};
use minecraft_protocol::frame::{ConnectionState, FrameDecoder, RawPacket};
use minecraft_protocol::types::position::write_string;
use minecraft_protocol::types::varint::write_varint;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{debug, error, info};

/// Background embedded server handle.
pub struct EmbeddedServer {
    addr: SocketAddr,
    _running: Arc<AtomicBool>,
}

impl EmbeddedServer {
    /// Returns the bound local socket address.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Spawns the embedded offline server on `127.0.0.1:25565` (or falls back to ephemeral port).
    pub async fn start() -> anyhow::Result<Self> {
        // Try standard Minecraft port first, fallback to an open ephemeral port
        let listener = match TcpListener::bind("127.0.0.1:25565").await {
            Ok(l) => l,
            Err(_) => TcpListener::bind("127.0.0.1:0").await?,
        };

        let addr = listener.local_addr()?;
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);

        info!("[Embedded Server] Started offline internal server on {addr}");

        tokio::spawn(async move {
            while running_clone.load(Ordering::Relaxed) {
                if let Ok((socket, peer_addr)) = listener.accept().await {
                    info!("[Embedded Server] Client connected from {peer_addr}");
                    tokio::spawn(handle_client(socket));
                }
            }
        });

        Ok(Self {
            addr,
            _running: running,
        })
    }
}

async fn handle_client(mut stream: tokio::net::TcpStream) {
    let (mut reader, mut writer) = stream.split();
    let mut buf = BytesMut::with_capacity(65536);
    let mut state = ConnectionState::Handshaking;

    let mut keepalive_interval = tokio::time::interval(Duration::from_secs(5));
    let mut keepalive_id: i32 = 1000;

    loop {
        tokio::select! {
            res = reader.read_buf(&mut buf) => {
                match res {
                    Ok(0) => break,
                    Ok(_) => {
                        while let Ok(Some(pkt)) = FrameDecoder::decode(&mut buf) {
                            debug!("[Embedded Server] Got packet 0x{:02X} in state {:?}", pkt.id, state);

                            match state {
                                ConnectionState::Handshaking if pkt.id == 0x00 => {
                                    // Handshake received, transition to Login
                                    state = ConnectionState::Login;
                                }
                                ConnectionState::Login if pkt.id == 0x00 => {
                                    // Login Start received -> send Login Success (0x02)
                                    let mut p = BytesMut::new();
                                    write_string(&mut p, "11111111-2222-3333-4444-555555555555");
                                    write_string(&mut p, "OfflinePlayer");
                                    let login_success = RawPacket::new(0x02, p.freeze());
                                    let _ = writer.write_all(&login_success.encode_uncompressed()).await;

                                    state = ConnectionState::Play;
                                    info!("[Embedded Server] Client logged in! Transitioned to Play state.");

                                    // Send S01 Join Game
                                    let mut s01 = BytesMut::new();
                                    s01.put_i32(100); // Entity ID
                                    s01.put_u8(0);    // Survival mode
                                    s01.put_i8(0);    // Overworld
                                    s01.put_u8(1);    // Easy difficulty
                                    s01.put_u8(20);   // Max players
                                    write_string(&mut s01, "default");
                                    s01.put_u8(0);    // reduced debug info = false
                                    let _ = writer.write_all(&RawPacket::new(0x01, s01.freeze()).encode_uncompressed()).await;

                                    // Send S05 Spawn Position (0, 64, 0)
                                    let mut s05 = BytesMut::new();
                                    let spawn_pos = minecraft_protocol::Position::new(0, 64, 0);
                                    minecraft_protocol::write_position(&mut s05, spawn_pos);
                                    let _ = writer.write_all(&RawPacket::new(0x05, s05.freeze()).encode_uncompressed()).await;

                                    // Send S08 Player Position & Look (spawn player at 0.5, 65.0, 0.5)
                                    let mut s08 = BytesMut::new();
                                    s08.put_f64(0.5);
                                    s08.put_f64(65.0);
                                    s08.put_f64(0.5);
                                    s08.put_f32(0.0);
                                    s08.put_f32(0.0);
                                    s08.put_u8(0); // absolute
                                    let _ = writer.write_all(&RawPacket::new(0x08, s08.freeze()).encode_uncompressed()).await;

                                    // Send synthetic sample terrain chunks (3x3 grid around spawn: -1..=1)
                                    for cx in -1..=1 {
                                        for cz in -1..=1 {
                                            let chunk_pkt = generate_sample_flat_chunk(cx, cz);
                                            let _ = writer.write_all(&chunk_pkt.encode_uncompressed()).await;
                                        }
                                    }
                                }
                                ConnectionState::Play if pkt.id == 0x00 => {
                                    // Client KeepAlive ACK received
                                    debug!("[Embedded Server] KeepAlive ACK received from client");
                                }
                                ConnectionState::Play if pkt.id == 0x06 => {
                                    // Client Position & Look
                                    debug!("[Embedded Server] Client Position ACK received");
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(e) => {
                        error!("[Embedded Server] Socket read error: {e}");
                        break;
                    }
                }
            }
            _ = keepalive_interval.tick() => {
                if state == ConnectionState::Play {
                    keepalive_id += 1;
                    let mut p = BytesMut::new();
                    write_varint(&mut p, keepalive_id);
                    let _ = writer.write_all(&RawPacket::new(0x00, p.freeze()).encode_uncompressed()).await;
                }
            }
        }
    }

    info!("[Embedded Server] Client session finished");
}

/// Generates a valid S21 Chunk Data packet representing a flat world chunk (bedrock, dirt, grass).
fn generate_sample_flat_chunk(chunk_x: i32, chunk_z: i32) -> RawPacket {
    let mut p = BytesMut::new();
    p.put_i32(chunk_x);
    p.put_i32(chunk_z);
    p.put_u8(1); // ground_up_continuous
    p.put_u16(1); // bitmask = Section 0 (Y=0..15)

    // Section 0 block payload: 4096 blocks * 2 bytes = 8192 bytes
    // Light: 2048 blocklight + 2048 skylight = 4096 bytes
    // Biomes: 256 bytes
    // Total = 12544 bytes
    let total_bytes: i32 = 8192 + 2048 + 2048 + 256;
    write_varint(&mut p, total_bytes);

    // Bedrock at Y=0 (ID 7), Dirt at Y=1..3 (ID 3), Grass at Y=4 (ID 2), Air above (ID 0)
    for y in 0..16 {
        let block_id: u16 = match y {
            0 => 7,     // Bedrock
            1..=3 => 3, // Dirt
            4 => 2,     // Grass
            _ => 0,     // Air
        };
        for _ in 0..(16 * 16) {
            p.put_u16_le(block_id << 4);
        }
    }

    // Block light: 2048 bytes of 0
    p.put_bytes(0, 2048);
    // Sky light: 2048 bytes of 0xFF (full skylight)
    p.put_bytes(0xFF, 2048);
    // Biomes: 256 bytes (Plains biome ID 1)
    p.put_bytes(1, 256);

    RawPacket::new(0x21, p.freeze())
}
