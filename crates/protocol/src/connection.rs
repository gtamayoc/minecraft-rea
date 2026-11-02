//! Asynchronous network connection client for Minecraft Java Edition 1.8.9.

use crate::error::{ProtocolError, Result};
use crate::frame::{ConnectionState, FrameDecoder, RawPacket};
use crate::packets::handshake::{HandshakePacket, LoginStartPacket, NextState};
use crate::packets::play::ClientKeepAlivePacket;
use bytes::BytesMut;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Events produced by the network connection loop.
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// Received a packet with ID and payload.
    PacketReceived(RawPacket),
    /// Connection state changed.
    StateChanged(ConnectionState),
    /// Disconnected from server.
    Disconnected(String),
}

/// Client network connection handle.
pub struct NetworkConnection {
    tx_packet: mpsc::Sender<RawPacket>,
    is_running: Arc<AtomicBool>,
}

impl NetworkConnection {
    /// Connects to a Minecraft server, performs Handshake and starts reading/writing loops.
    pub async fn connect(
        addr: SocketAddr,
        username: &str,
        event_tx: mpsc::Sender<NetworkEvent>,
    ) -> Result<Self> {
        info!("Connecting to Minecraft server at {addr} as {username}...");
        let stream = TcpStream::connect(addr)
            .await
            .map_err(ProtocolError::DecompressionError)?;

        let (mut reader, mut writer) = stream.into_split();
        let (tx_packet, mut rx_packet) = mpsc::channel::<RawPacket>(100);
        let is_running = Arc::new(AtomicBool::new(true));

        // 1. Send Handshake
        let host = addr.ip().to_string();
        let handshake = HandshakePacket::new(host, addr.port(), NextState::Login);
        let handshake_bytes = handshake.to_raw_packet().encode_uncompressed();
        writer
            .write_all(&handshake_bytes)
            .await
            .map_err(ProtocolError::DecompressionError)?;

        // 2. Send Login Start
        let login_start = LoginStartPacket::new(username);
        let login_bytes = login_start.to_raw_packet().encode_uncompressed();
        writer
            .write_all(&login_bytes)
            .await
            .map_err(ProtocolError::DecompressionError)?;

        let running_clone = Arc::clone(&is_running);
        let event_tx_clone = event_tx.clone();

        // Background write task
        tokio::spawn(async move {
            while let Some(pkt) = rx_packet.recv().await {
                let encoded = pkt.encode_uncompressed();
                if let Err(e) = writer.write_all(&encoded).await {
                    error!("Failed to write packet: {e}");
                    break;
                }
            }
        });

        // Background read task
        let tx_packet_clone = tx_packet.clone();
        tokio::spawn(async move {
            let mut buf = BytesMut::with_capacity(65536);
            let mut state = ConnectionState::Login;

            while running_clone.load(Ordering::Relaxed) {
                match reader.read_buf(&mut buf).await {
                    Ok(0) => {
                        info!("Server closed connection");
                        let _ = event_tx_clone
                            .send(NetworkEvent::Disconnected("Server closed".to_string()))
                            .await;
                        break;
                    }
                    Ok(_) => {
                        while let Ok(Some(pkt)) = FrameDecoder::decode(&mut buf) {
                            debug!("Received packet 0x{:02X} in state {:?}", pkt.id, state);

                            // Handle protocol handshakes automatically
                            match state {
                                ConnectionState::Login if pkt.id == 0x02 => {
                                    info!("Login success received!");
                                    state = ConnectionState::Play;
                                    let _ = event_tx_clone
                                        .send(NetworkEvent::StateChanged(state))
                                        .await;
                                }
                                ConnectionState::Play if pkt.id == 0x00 => {
                                    // KeepAlive
                                    let mut p = pkt.payload.clone();
                                    if let Ok(id) = crate::types::varint::read_varint(&mut p) {
                                        debug!("Responding to keepalive {id}");
                                        let resp = ClientKeepAlivePacket::new(id).to_raw_packet();
                                        let _ = tx_packet_clone.send(resp).await;
                                    }
                                }
                                _ => {}
                            }

                            let _ = event_tx_clone.send(NetworkEvent::PacketReceived(pkt)).await;
                        }
                    }
                    Err(e) => {
                        error!("TCP read error: {e}");
                        let _ = event_tx_clone
                            .send(NetworkEvent::Disconnected(e.to_string()))
                            .await;
                        break;
                    }
                }
            }
            running_clone.store(false, Ordering::Relaxed);
        });

        Ok(Self {
            tx_packet,
            is_running,
        })
    }

    /// Sends a packet to the server.
    pub async fn send_packet(&self, packet: RawPacket) -> Result<()> {
        self.tx_packet
            .send(packet)
            .await
            .map_err(|_| ProtocolError::UnexpectedEof {
                needed: 0,
                available: 0,
            })?;
        Ok(())
    }

    /// Checks if the connection is active.
    pub fn is_alive(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }
}
