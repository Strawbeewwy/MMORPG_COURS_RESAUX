//! Async network client exposed to Godot as a `NetworkClient` Node.
//!
//! Architecture
//! ────────────
//! The Godot main thread must never block.  A dedicated OS thread hosts a
//! Tokio runtime for all I/O.  Thread-safe channels bridge the two worlds:
//!
//!   Tokio thread                      Godot thread
//!   ──────────────                    ────────────────────
//!   TcpStream read  → inbox (Mutex)  → process() → emit_signal
//!   outbox (Mutex) ← send_position() ←  GDScript func call
//!
//! Reconnection uses exponential back-off (1 s → 32 s, capped).
use godot::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

// Re-export so world/ and ui/ can reach the shared inbox type.
pub type Inbox = Arc<Mutex<Vec<IncomingEvent>>>;
pub type Outbox = Arc<Mutex<Vec<Vec<u8>>>>;

/// Simplified event type decoded from the broker wire format.
/// Extend as new server messages are handled.
#[derive(Debug, Clone)]
pub enum IncomingEvent {
    PositionUpdate { client_id: i64, x: f32, y: f32 },
    PlayerJoined { client_id: i64 },
    PlayerLeft { client_id: i64 },
}

// ─── NetworkClient Node ───────────────────────────────────────────────────────

/// Godot autoload node.  Add it to the scene tree as an autoload singleton
/// named `NetworkClient` and connect its signals from GDScript.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct NetworkClient {
    base: Base<Node>,
    inbox: Inbox,
    outbox: Outbox,
    /// Authenticated client id — 0 until login completes.
    my_client_id: i64,
    /// Server address read from Godot project settings or defaulting to localhost.
    server_addr: String,
}

#[godot_api]
impl INode for NetworkClient {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            inbox: Arc::new(Mutex::new(Vec::new())),
            outbox: Arc::new(Mutex::new(Vec::new())),
            my_client_id: 0,
            server_addr: "127.0.0.1:9000".to_string(),
        }
    }

    fn ready(&mut self) {
        // Optional: read server address from project settings.
        // let addr = ProjectSettings::singleton()
        //     .get_setting("network/server_address".into())
        //     .to::<GString>().to_string();

        let inbox = self.inbox.clone();
        let outbox = self.outbox.clone();
        let addr = self.server_addr.clone();

        // Spawn the I/O thread.  It owns its own Tokio runtime.
        std::thread::Builder::new()
            .name("mmo_network".to_string())
            .spawn(move || {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("tokio runtime")
                    .block_on(run_with_reconnect(addr, inbox, outbox));
            })
            .expect("spawn network thread");

        tracing::info!("NetworkClient ready — connecting to {}", self.server_addr);
    }

    /// Pump inbox into Godot signals — called every frame.
    fn process(&mut self, _delta: f64) {
        let events: Vec<IncomingEvent> = {
            let mut guard = self.inbox.lock().unwrap();
            guard.drain(..).collect()
        };

        for event in events {
            match event {
                IncomingEvent::PositionUpdate { client_id, x, y } => {
                    self.base_mut().emit_signal(
                        "position_received",
                        &[
                            client_id.to_variant(),
                            x.to_variant(),
                            y.to_variant(),
                        ],
                    );
                }
                IncomingEvent::PlayerJoined { client_id } => {
                    self.base_mut()
                        .emit_signal("player_joined", &[client_id.to_variant()]);
                }
                IncomingEvent::PlayerLeft { client_id } => {
                    self.base_mut()
                        .emit_signal("player_left", &[client_id.to_variant()]);
                }
            }
        }
    }
}

#[godot_api]
impl NetworkClient {
    // ── Signals emitted toward GDScript ──────────────────────────────────────

    /// Emitted every time a position update is received for any client.
    #[signal]
    fn position_received(client_id: i64, x: f32, y: f32);

    /// Emitted when a new player enters the area of interest.
    #[signal]
    fn player_joined(client_id: i64);

    /// Emitted when a player leaves the area of interest or disconnects.
    #[signal]
    fn player_left(client_id: i64);

    // ── Functions callable from GDScript ─────────────────────────────────────

    /// Set the authenticated client id after login.
    #[func]
    fn set_client_id(&mut self, id: i64) {
        self.my_client_id = id;
        tracing::info!("NetworkClient: local client_id set to {id}");
    }

    /// Send the local player's position to the server.
    /// Called from `_physics_process` in player.gd.
    #[func]
    fn send_position(&self, x: f32, y: f32) {
        // Wire format: tag 0x01 | client_id (8 bytes LE) | x (4 bytes LE) | y (4 bytes LE)
        let mut payload = Vec::with_capacity(17);
        payload.push(0x01u8);
        payload.extend_from_slice(&self.my_client_id.to_le_bytes());
        payload.extend_from_slice(&x.to_le_bytes());
        payload.extend_from_slice(&y.to_le_bytes());
        self.outbox.lock().unwrap().push(payload);
    }

    /// Override the server address before `ready()` is called.
    #[func]
    fn set_server_addr(&mut self, addr: GString) {
        self.server_addr = addr.to_string();
    }
}

// ─── Async I/O ───────────────────────────────────────────────────────────────

/// Connect and reconnect with exponential back-off.
async fn run_with_reconnect(addr: String, inbox: Inbox, outbox: Outbox) {
    let mut backoff = Duration::from_secs(1);
    loop {
        match TcpStream::connect(&addr).await {
            Ok(stream) => {
                tracing::info!("network: connected to {addr}");
                backoff = Duration::from_secs(1); // reset on success
                run_connection(stream, &inbox, &outbox).await;
                tracing::warn!("network: disconnected from {addr}");
            }
            Err(e) => {
                tracing::error!("network: cannot connect to {addr}: {e} — retry in {backoff:?}");
            }
        }
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_secs(32));
    }
}

/// I/O loop for a live connection.  Returns when the connection drops.
async fn run_connection(mut stream: TcpStream, inbox: &Inbox, outbox: &Outbox) {
    let mut buf = vec![0u8; 8192];
    loop {
        // Drain outbound queue first.
        let pending: Vec<Vec<u8>> = outbox.lock().unwrap().drain(..).collect();
        for payload in pending {
            if stream.write_all(&payload).await.is_err() {
                return;
            }
        }

        // Wait for incoming bytes or a short timeout to re-check outbox.
        tokio::select! {
            result = stream.read(&mut buf) => {
                match result {
                    Ok(0) => return, // server closed connection
                    Ok(n) => decode_and_push(&buf[..n], inbox),
                    Err(_) => return,
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(8)) => {
                // Timeout — loop to flush outbox.
            }
        }
    }
}

/// Minimal wire decoder — mirrors `Shared/src/protocol`.
/// Tag 0x02: PositionUpdate | client_id (8) | x (4) | y (4)
/// Tag 0x03: PlayerJoined   | client_id (8)
/// Tag 0x04: PlayerLeft     | client_id (8)
fn decode_and_push(data: &[u8], inbox: &Inbox) {
    let mut cursor = 0;
    while cursor < data.len() {
        let tag = data[cursor];
        cursor += 1;
        let event = match tag {
            0x02 if data.len() >= cursor + 16 => {
                let client_id = i64::from_le_bytes(data[cursor..cursor + 8].try_into().unwrap());
                let x = f32::from_le_bytes(data[cursor + 8..cursor + 12].try_into().unwrap());
                let y = f32::from_le_bytes(data[cursor + 12..cursor + 16].try_into().unwrap());
                cursor += 16;
                Some(IncomingEvent::PositionUpdate { client_id, x, y })
            }
            0x03 if data.len() >= cursor + 8 => {
                let client_id = i64::from_le_bytes(data[cursor..cursor + 8].try_into().unwrap());
                cursor += 8;
                Some(IncomingEvent::PlayerJoined { client_id })
            }
            0x04 if data.len() >= cursor + 8 => {
                let client_id = i64::from_le_bytes(data[cursor..cursor + 8].try_into().unwrap());
                cursor += 8;
                Some(IncomingEvent::PlayerLeft { client_id })
            }
            _ => break, // unknown tag — discard remainder
        };
        if let Some(ev) = event {
            inbox.lock().unwrap().push(ev);
        }
    }
}

