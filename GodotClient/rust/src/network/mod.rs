//! QUIC network client for the Godot MMO frontend.
//!
//! Uses the same `game_sockets::GamePeer` / `QuicBackend` as every Bevy actor
//! (GameServer, SpatialService) so the Broker sees no difference.
//!
//! Wire protocol is byte-for-byte identical to `Shared/src/protocol/broker`:
//!   send  ClientHello  → 0x08 | u16_le(username.len) | username_bytes
//!   send  ClientInput  → 0x05 | u32_le(client_id) | f32_le(x) | f32_le(y) | [0;8]
//!   recv  ClientAccepted → 0x0A | u32_le(client_id)
//!   recv  Broadcast    → 0x04 | u16_le(payload_len) | payload
//!
//! Threading model
//! ───────────────
//!   Godot main thread          │  network thread (std::thread + Tokio rt)
//!   ─────────────────────────  │  ─────────────────────────────────────────
//!   NetworkClient::process()   │  poll_loop()
//!     drain inbox → signals    │    peer.poll() → push IncomingEvent to inbox
//!     drain outbox → peer.send │    take outbox → peer.send

use bytes::Bytes;
use game_sockets::{
    protocols::QuicBackend, GameNetworkEvent, GamePeer, GameStream,
    GameStreamReliability,
};
use godot::classes::INode;
use godot::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ── Wire protocol tags (mirrors Shared/src/protocol/broker/config.rs) ─────────
const TAG_CLIENT_HELLO:   u8 = 0x08;
const TAG_CLIENT_INPUT:   u8 = 0x05;
const TAG_CLIENT_ACCEPTED:u8 = 0x0A;
const TAG_BROADCAST:      u8 = 0x04;
/// Size of the ClientInput payload array (mirrors CLIENT_INPUT_LEN in Shared).
/// Layout: [move_x f32][move_y f32][action_flags u8][look_x f32][look_y f32] = 17 bytes.
const CLIENT_INPUT_LEN: usize = 17;

// ── Shared channel types ───────────────────────────────────────────────────────
pub type Inbox  = Arc<Mutex<Vec<IncomingEvent>>>;
pub type Outbox = Arc<Mutex<Vec<Vec<u8>>>>;

/// Events decoded from Broker messages and forwarded to the Godot main thread.
#[derive(Debug, Clone)]
pub enum IncomingEvent {
    /// Broker assigned us a client_id after ClientHello.
    ClientAccepted { client_id: u32 },
    /// GameServer broadcast — contains a serialised WorldUpdate payload.
    Broadcast { payload: Vec<u8> },
    /// Global colour swap fired.  `swap_index` even = Red, odd = Blue.
    ColorSwap { swap_index: u64 },
    /// Server assigned this client a colour team.
    PlayerColorAssigned { client_id: u32, color_team: u8 },
    /// Batch enemy state for this tick (flat interleaved array: id, x, y, color, hp).
    EnemiesUpdate { data: Vec<f32> },
    /// An enemy was killed.
    EnemyDied { enemy_id: u32 },
    /// Batch projectile state (flat: id, x, y, dx, dy, color, alive).
    ProjectilesUpdate { data: Vec<f32> },
    /// Cumulative score for a player.
    PlayerScoreUpdate { client_id: u32, score: u32 },
}

// ── Wire helpers ───────────────────────────────────────────────────────────────

/// Encode a ClientHello packet (TAG_CLIENT_HELLO).
pub fn encode_client_hello(username: &str) -> Vec<u8> {
    let name = username.as_bytes();
    let mut buf = Vec::with_capacity(1 + 2 + name.len());
    buf.push(TAG_CLIENT_HELLO);
    buf.extend_from_slice(&(name.len() as u16).to_le_bytes());
    buf.extend_from_slice(name);
    buf
}

/// Encode a ClientInput packet (TAG_CLIENT_INPUT) with movement x/y.
/// Matches `encode_movement_input` in GameClient/src/net/input.rs.
pub fn encode_client_input(client_id: u32, x: f32, y: f32) -> Vec<u8> {
    encode_full_input(client_id, x, y, 0, x, y)
}

/// Full input packet: movement + action flags + look direction.
/// Payload layout (CLIENT_INPUT_LEN = 17 bytes):
///   [0..4]   move_x f32 LE
///   [4..8]   move_y f32 LE
///   [8]      action_flags bitmask (bit0=dash, bit1=melee, bit2=shoot)
///   [9..13]  look_x f32 LE
///   [13..17] look_y f32 LE
pub fn encode_full_input(
    client_id: u32,
    move_x: f32, move_y: f32,
    action_flags: u8,
    look_x: f32, look_y: f32,
) -> Vec<u8> {
    let mut input = [0u8; CLIENT_INPUT_LEN];
    input[0..4].copy_from_slice(&move_x.to_le_bytes());
    input[4..8].copy_from_slice(&move_y.to_le_bytes());
    input[8] = action_flags;
    input[9..13].copy_from_slice(&look_x.to_le_bytes());
    input[13..17].copy_from_slice(&look_y.to_le_bytes());

    let mut buf = Vec::with_capacity(1 + 4 + CLIENT_INPUT_LEN);
    buf.push(TAG_CLIENT_INPUT);
    buf.extend_from_slice(&client_id.to_le_bytes());
    buf.extend_from_slice(&input);
    buf
}

/// Decode a single Broker message from raw bytes.
fn decode(data: &[u8]) -> Option<IncomingEvent> {
    use shared::protocol::WorldUpdate;

    let (&tag, body) = data.split_first()?;
    match tag {
        TAG_CLIENT_ACCEPTED if body.len() >= 4 => {
            let client_id = u32::from_le_bytes(body[0..4].try_into().ok()?);
            Some(IncomingEvent::ClientAccepted { client_id })
        }
        TAG_BROADCAST if body.len() >= 2 => {
            let declared_len = u16::from_le_bytes(body[0..2].try_into().ok()?) as usize;
            let payload = body[2..2 + declared_len.min(body.len().saturating_sub(2))].to_vec();

            // Try to deserialise as WorldUpdate.
            match shared::protocol::transport::codec::decode::<WorldUpdate>(&payload) {
                Ok(update) => decode_world_update(update),
                Err(_) => {
                    // Unknown payload — forward raw bytes.
                    Some(IncomingEvent::Broadcast { payload })
                }
            }
        }
        _ => {
            tracing::debug!("mmo_client: ignoring broker tag 0x{tag:02X} ({} bytes)", body.len());
            None
        }
    }
}

fn decode_world_update(update: shared::protocol::WorldUpdate) -> Option<IncomingEvent> {
    use shared::protocol::WorldUpdate;
    match update {
        WorldUpdate::ColorSwap { swap_index } => {
            Some(IncomingEvent::ColorSwap { swap_index })
        }
        WorldUpdate::PlayerColorAssigned { client_id, color } => {
            Some(IncomingEvent::PlayerColorAssigned {
                client_id: client_id.0,
                color_team: color as u8,
            })
        }
        WorldUpdate::EnemiesUpdate { enemies } => {
            // Pack into flat f32 array: [id, x, y, color, hp] × n
            let mut data = Vec::with_capacity(enemies.len() * 5);
            for e in &enemies {
                let (x, y) = e.position.to_f32();
                data.push(e.id as f32);
                data.push(x);
                data.push(y);
                data.push(e.color as u8 as f32);
                data.push(e.hp as f32);
            }
            Some(IncomingEvent::EnemiesUpdate { data })
        }
        WorldUpdate::EnemyDied { enemy_id, .. } => {
            Some(IncomingEvent::EnemyDied { enemy_id })
        }
        WorldUpdate::ProjectilesUpdate { projectiles } => {
            let mut data = Vec::with_capacity(projectiles.len() * 7);
            for p in &projectiles {
                let (px, py) = p.position.to_f32();
                let (dx, dy) = p.direction.to_f32();
                data.push(p.id as f32);
                data.push(px);
                data.push(py);
                data.push(dx);
                data.push(dy);
                data.push(p.color as u8 as f32);
                data.push(if p.alive { 1.0 } else { 0.0 });
            }
            Some(IncomingEvent::ProjectilesUpdate { data })
        }
        WorldUpdate::PlayerScoreUpdate { client_id, score } => {
            Some(IncomingEvent::PlayerScoreUpdate {
                client_id: client_id.0,
                score,
            })
        }
        // Legacy variants — forward as raw broadcast.
        legacy => {
            if let Ok(payload) = shared::protocol::transport::codec::encode(&legacy) {
                Some(IncomingEvent::Broadcast { payload: payload.to_vec() })
            } else {
                None
            }
        }
    }
}

// ── NetworkClient Godot node ───────────────────────────────────────────────────

/// Godot autoload node — add to the scene tree as `NetworkClient`.
/// Exposes signals and functions to GDScript; handles the QUIC connection
/// to the Broker on a dedicated thread.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct NetworkClient {
    base:      Base<Node>,
    inbox:     Inbox,
    outbox:    Outbox,
    client_id: u32,
    server_addr: String,
}

#[godot_api]
impl INode for NetworkClient {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            inbox:  Arc::new(Mutex::new(Vec::new())),
            outbox: Arc::new(Mutex::new(Vec::new())),
            client_id: 0,
            server_addr: "127.0.0.1:9600".to_string(),
        }
    }

    fn ready(&mut self) {
        let inbox  = self.inbox.clone();
        let outbox = self.outbox.clone();
        let addr   = self.server_addr.clone();

        std::thread::Builder::new()
            .name("mmo_quic".into())
            .spawn(move || {
                // quinn (inside game_sockets) requires a Tokio runtime.
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("tokio rt")
                    .block_on(async {
                        poll_loop(addr, inbox, outbox).await;
                    });
            })
            .expect("spawn mmo_quic thread");

        tracing::info!("NetworkClient ready — broker {}", self.server_addr);
    }

    /// Pump inbox into Godot signals each frame.
    fn process(&mut self, _delta: f64) {
        let events: Vec<IncomingEvent> = {
            let mut g = self.inbox.lock().unwrap();
            g.drain(..).collect()
        };
        for ev in events {
            match ev {
                IncomingEvent::ClientAccepted { client_id } => {
                    self.client_id = client_id;
                    tracing::info!("NetworkClient: assigned client_id={client_id}");
                    self.base_mut().emit_signal(
                        "client_accepted",
                        &[(client_id as i64).to_variant()],
                    );
                }
                IncomingEvent::Broadcast { payload } => {
                    let bytes: PackedByteArray = payload.iter().copied().collect();
                    self.base_mut().emit_signal("broadcast_received", &[bytes.to_variant()]);
                }
                IncomingEvent::ColorSwap { swap_index } => {
                    // Even swap_index → Red (0), odd → Blue (1).
                    let team: i64 = (swap_index % 2) as i64;
                    self.base_mut().emit_signal(
                        "color_swapped",
                        &[(swap_index as i64).to_variant(), team.to_variant()],
                    );
                }
                IncomingEvent::PlayerColorAssigned { client_id, color_team } => {
                    self.base_mut().emit_signal(
                        "player_color_assigned",
                        &[(client_id as i64).to_variant(), (color_team as i64).to_variant()],
                    );
                }
                IncomingEvent::EnemiesUpdate { data } => {
                    let arr: PackedFloat32Array = data.into_iter().collect();
                    self.base_mut().emit_signal("enemies_updated", &[arr.to_variant()]);
                }
                IncomingEvent::EnemyDied { enemy_id } => {
                    self.base_mut().emit_signal(
                        "enemy_died",
                        &[(enemy_id as i64).to_variant()],
                    );
                }
                IncomingEvent::ProjectilesUpdate { data } => {
                    let arr: PackedFloat32Array = data.into_iter().collect();
                    self.base_mut().emit_signal("projectiles_updated", &[arr.to_variant()]);
                }
                IncomingEvent::PlayerScoreUpdate { client_id, score } => {
                    self.base_mut().emit_signal(
                        "score_updated",
                        &[(client_id as i64).to_variant(), (score as i64).to_variant()],
                    );
                }
            }
        }
    }
}

#[godot_api]
impl NetworkClient {
    // ── Signals ───────────────────────────────────────────────────────────────

    /// Broker accepted the connection and assigned a client_id.
    #[signal]
    fn client_accepted(client_id: i64);

    /// A WorldUpdate broadcast from the GameServer was received.
    #[signal]
    fn broadcast_received(payload: PackedByteArray);

    /// Global colour swap — swap_index (i64), color_team (0=Red, 1=Blue).
    #[signal]
    fn color_swapped(swap_index: i64, color_team: i64);

    /// Server assigned a colour team to a player.
    #[signal]
    fn player_color_assigned(client_id: i64, color_team: i64);

    /// Batch enemy positions/state as PackedFloat32Array.
    /// Layout per enemy: [id_f32, x, y, color_f32, hp_f32]  (5 floats/enemy).
    #[signal]
    fn enemies_updated(data: PackedFloat32Array);

    /// An enemy was killed (removed from simulation).
    #[signal]
    fn enemy_died(enemy_id: i64);

    /// Batch projectile state as PackedFloat32Array.
    /// Layout: [id, x, y, dx, dy, color, alive]  (7 floats/projectile).
    #[signal]
    fn projectiles_updated(data: PackedFloat32Array);

    /// Cumulative score update for a player.
    #[signal]
    fn score_updated(client_id: i64, score: i64);

    // ── GDScript-callable functions ────────────────────────────────────────────

    /// Send the local player's movement direction to the Broker.
    /// x / y are normalised direction values in [-1, 1].
    #[func]
    fn send_movement(&self, x: f32, y: f32) {
        let pkt = encode_client_input(self.client_id, x, y);
        self.outbox.lock().unwrap().push(pkt);
    }

    /// Send movement + action flags + look direction in one packet.
    /// action_flags bitmask: bit0=dash, bit1=melee, bit2=shoot.
    #[func]
    fn send_action_input(
        &self,
        move_x: f32,
        move_y: f32,
        action_flags: i32,
        look_x: f32,
        look_y: f32,
    ) {
        let pkt = encode_full_input(
            self.client_id,
            move_x, move_y,
            action_flags as u8,
            look_x, look_y,
        );
        self.outbox.lock().unwrap().push(pkt);
    }

    /// Override the Broker address before the node enters the scene tree.
    #[func]
    fn set_broker_addr(&mut self, addr: GString) {
        self.server_addr = addr.to_string();
    }

    /// Return the client_id assigned by the Broker (0 until ClientAccepted).
    #[func]
    fn get_client_id(&self) -> i64 {
        self.client_id as i64
    }
}

// ── Background QUIC poll loop ──────────────────────────────────────────────────

async fn poll_loop(addr: String, inbox: Inbox, outbox: Outbox) {
    let host_port: Vec<&str> = addr.rsplitn(2, ':').collect();
    let (port, host) = match host_port.as_slice() {
        [p, h] => (p.parse::<u16>().unwrap_or(9600), *h),
        _ => {
            tracing::error!("invalid broker addr: {addr}");
            return;
        }
    };

    let mut backoff = Duration::from_secs(1);
    loop {
        match try_connect_and_run(host, port, &inbox, &outbox).await {
            Ok(()) => tracing::warn!("mmo_quic: disconnected from broker — reconnecting"),
            Err(e) => tracing::error!("mmo_quic: connection error: {e} — retry in {backoff:?}"),
        }
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_secs(32));
    }
}

async fn try_connect_and_run(
    host:   &str,
    port:   u16,
    inbox:  &Inbox,
    outbox: &Outbox,
) -> anyhow::Result<()> {
    let peer = GamePeer::new(QuicBackend::new());
    peer.connect(host, port)?;
    tracing::info!("mmo_quic: connecting to broker {host}:{port}");

    let mut peer = peer;
    let mut connection: Option<game_sockets::GameConnection> = None;
    let mut stream:     Option<GameStream> = None;
    let mut hello_sent = false;

    loop {
        loop {
            match peer.poll() {
                Ok(Some(event)) => handle_event(
                    &mut peer, event,
                    &mut connection, &mut stream, &mut hello_sent,
                    inbox,
                ),
                Ok(None) => break,
                Err(e) => {
                    tracing::warn!("mmo_quic: poll error: {e}");
                    return Ok(());
                }
            }
        }

        if let (Some(conn), Some(st)) = (connection.as_ref(), stream.as_ref()) {
            let pending: Vec<Vec<u8>> = {
                let mut g = outbox.lock().unwrap();
                g.drain(..).collect()
            };
            for pkt in pending {
                if let Err(e) = peer.send(conn, st, Bytes::from(pkt)) {
                    tracing::warn!("mmo_quic: send error: {e}");
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(8)).await;
    }
}

fn handle_event(
    peer:        &mut GamePeer,
    event:       GameNetworkEvent,
    connection:  &mut Option<game_sockets::GameConnection>,
    stream:      &mut Option<GameStream>,
    hello_sent:  &mut bool,
    inbox:       &Inbox,
) {
    match event {
        GameNetworkEvent::Connected(conn) => {
            tracing::info!("mmo_quic: connected (id={})", conn.connection_id);
            *connection = Some(conn);
            // Open a reliable stream — Broker expects one per client.
            if let Err(e) = peer.create_stream(conn, GameStreamReliability::Reliable) {
                tracing::error!("mmo_quic: create_stream failed: {e}");
            }
        }

        GameNetworkEvent::StreamCreated(conn, st) if st.is_reliable() => {
            tracing::info!("mmo_quic: reliable stream ready (stream={})", st.stream_id);
            // Send ClientHello before storing — borrow st before the move.
            if !*hello_sent {
                let hello = encode_client_hello("GodotPlayer");
                if let Err(e) = peer.send(&conn, &st, Bytes::from(hello)) {
                    tracing::error!("mmo_quic: failed to send ClientHello: {e}");
                } else {
                    *hello_sent = true;
                    tracing::info!("mmo_quic: ClientHello sent");
                }
            }
            *stream = Some(st);
        }

        GameNetworkEvent::Message { data, .. } => {
            if let Some(ev) = decode(&data) {
                inbox.lock().unwrap().push(ev);
            }
        }

        GameNetworkEvent::Disconnected(_) => {
            tracing::warn!("mmo_quic: disconnected");
            *connection = None;
            *stream     = None;
            *hello_sent = false;
        }

        GameNetworkEvent::StreamClosed(_, _) => {
            *stream     = None;
            *hello_sent = false;
        }

        GameNetworkEvent::Error { inner, .. } => {
            tracing::warn!("mmo_quic: socket error: {inner}");
        }

        _ => {}
    }
}
