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

use shared::{
    CLIENT_INPUT_LEN,
    NetworkMessage,encode_message,
    decode_message,
    Username, ClientId,
};
use shared::game_sockets::*;

use godot::classes::INode;
use godot::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use shared::game_sockets::protocols::QuicBackend;

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
    /// Entity position update (for interpolation/dead reckoning).
    PositionUpdate { entity_id: u32, x: f32, y: f32 },
    /// A new player/entity joined the game.
    PlayerJoined { client_id: u32, entity_id: u32, x: f32, y: f32 },
    /// A player/entity left the game.
    PlayerLeft { entity_id: u32 },
}

// ── Wire helpers ───────────────────────────────────────────────────────────────

/// Encode a ClientHello packet (TAG_CLIENT_HELLO).
pub fn encode_client_hello(username: &str) -> Vec<u8> {
    let arc_username: Username = Arc::from(username.to_string());
    let packet = match encode_message(&NetworkMessage::ClientHello {
        username: arc_username,
    }) {
        Ok(packet) => packet,
        Err(error) => {
            tracing::warn!(
                "failed to encode ClientAccepted for user {}: {}",
                username.to_string(),
                error
            );
            return Vec::new();
        }
    };
    packet
}



pub fn encode_register_client(client_id: u32, username: &str) -> Vec<u8> {
    let username: Username = Arc::from(username.to_string());

    let packet = match encode_message(&NetworkMessage::RegisterClient {
        client_id: ClientId(client_id),
        username: username.clone(),
    }) {
        Ok(packet) => packet,
        Err(error) => {
            tracing::warn!(
                "failed to encode RegisterClient for client_id={} username={}: {}",
                client_id,
                username,
                error
            );
            return Vec::new();
        }
    };

    packet
}


/// Encode a ClientInput packet (TAG_CLIENT_INPUT) with movement x/y.
/// Matches `encode_movement_input` in GameClient/src/net/input.rs.
pub fn encode_client_input(client_id: u32, x: f32, y: f32) -> Vec<u8> {
    let mut input = [0_u8; CLIENT_INPUT_LEN];

    input[0..4].copy_from_slice(&x.to_le_bytes());
    input[4..8].copy_from_slice(&y.to_le_bytes());

    let packet = match encode_message(&NetworkMessage::ClientInput {
        client_id : ClientId(client_id),
        input,
    }) {
        Ok(packet) => packet,
        Err(error) => {
            tracing::warn!(
                "cannot encode ClientInput for client {}: {}",
                client_id,
                error
            );
            return Vec::new();
        }
    };

    packet
}

/// Encode a full input packet with movement, actions, and look direction.
/// action_flags bitmask: bit0=dash, bit1=melee, bit2=shoot.
pub fn encode_full_input(
    client_id: u32,
    move_x: f32,
    move_y: f32,
    action_flags: u8,
    look_x: f32,
    look_y: f32,
) -> Vec<u8> {
    let mut input = [0_u8; CLIENT_INPUT_LEN];

    // Movement direction (normalized)
    input[0..4].copy_from_slice(&move_x.to_le_bytes());
    input[4..8].copy_from_slice(&move_y.to_le_bytes());
    
    // Action flags
    input[8] = action_flags;
    
    // Look direction (normalized)
    input[9..13].copy_from_slice(&look_x.to_le_bytes());
    input[13..17].copy_from_slice(&look_y.to_le_bytes());

    let packet = match encode_message(&NetworkMessage::ClientInput {
        client_id: ClientId(client_id),
        input,
    }) {
        Ok(packet) => packet,
        Err(error) => {
            tracing::warn!(
                "cannot encode full ClientInput for client {}: {}",
                client_id,
                error
            );
            return Vec::new();
        }
    };

    packet
}

/// Decode a single Broker message from raw bytes.
fn decode(data: &[u8]) -> Option<IncomingEvent> {

    let message = match decode_message(data) {
        Ok(message) => message,
        Err(error) => {
            tracing::warn!(
                "could not decode message: {error}"
            );
            return None;
        }
    };

    match message{
        NetworkMessage::ClientAccepted { client_id } => {
            Some(IncomingEvent::ClientAccepted { client_id: client_id.0 })
        }
        NetworkMessage::Broadcast { payload, payload_len: _ } => {
            Some(IncomingEvent::Broadcast { payload })
        }
        NetworkMessage::RegisterEntity { entity_id, client_id, position } => {
            let (x, y) = position.to_f32();
            Some(IncomingEvent::PlayerJoined {
                client_id: client_id.0,
                entity_id: entity_id.0,
                x,
                y,
            })
        }
        NetworkMessage::PositionUpdate { entity_id, position } => {
            let (x, y) = position.to_f32();
            Some(IncomingEvent::PositionUpdate {
                entity_id: entity_id.0,
                x,
                y,
            })
        }
        NetworkMessage::UnregisterEntity { entity_id } => {
            Some(IncomingEvent::PlayerLeft {
                entity_id: entity_id.0,
            })
        }
        _ => {
            tracing::debug!("mmo_client: ignoring message {:?}", message);
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
        // Unimplemented variants — ignored for now
        WorldUpdate::Snapshot { .. }
        | WorldUpdate::PlayerJoined { .. }
        | WorldUpdate::PlayerLeft { .. } => {
            tracing::debug!("mmo_client: ignoring WorldUpdate variant");
            None
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
    username: String,
    server_addr: String,
}

#[godot_api]
impl INode for NetworkClient {
    fn init(base: Base<Node>) -> Self {
        let broker_host = std::env::var("BROKER_HOST")
            .unwrap_or_else(|_| "127.0.0.1".to_string());

        let broker_port = std::env::var("BROKER_PORT")
            .unwrap_or_else(|_| "9600".to_string());

        let client_id = std::env::var("CLIENT_ID")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(0);

        let username = std::env::var("USERNAME")
            .unwrap_or_else(|_| "GodotPlayer".to_string());

        Self {
            base,
            inbox:  Arc::new(Mutex::new(Vec::new())),
            outbox: Arc::new(Mutex::new(Vec::new())),
            client_id,
            username,
            server_addr: format!("{broker_host}:{broker_port}"),
        }
    }

    fn ready(&mut self) {
        let inbox  = self.inbox.clone();
        let outbox = self.outbox.clone();
        let addr   = self.server_addr.clone();
        let client_id = self.client_id;
        let username = self.username.clone();

        std::thread::Builder::new()
            .name("mmo_quic".into())
            .spawn(move || {
                // quinn (inside game_sockets) requires a Tokio runtime.
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("tokio rt")
                    .block_on(async {
                        poll_loop(addr, client_id, username, inbox, outbox).await;
                    });
            })
            .expect("spawn mmo_quic thread");

        tracing::info!(
            "NetworkClient ready — broker {} client_id={} username={}",
            self.server_addr,
            self.client_id,
            self.username
        );
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
                IncomingEvent::PositionUpdate { entity_id, x, y } => {
                    self.base_mut().emit_signal(
                        "position_received",
                        &[(entity_id as i64).to_variant(), x.to_variant(), y.to_variant()],
                    );
                }
                IncomingEvent::PlayerJoined { client_id, entity_id, x, y } => {
                    self.base_mut().emit_signal(
                        "player_joined",
                        &[
                            (client_id as i64).to_variant(),
                            (entity_id as i64).to_variant(),
                            x.to_variant(),
                            y.to_variant(),
                        ],
                    );
                }
                IncomingEvent::PlayerLeft { entity_id } => {
                    self.base_mut().emit_signal(
                        "player_left",
                        &[(entity_id as i64).to_variant()],
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

    /// Entity position update received.
    #[signal]
    fn position_received(entity_id: i64, x: f32, y: f32);

    /// A player/entity joined the game.
    #[signal]
    fn player_joined(client_id: i64, entity_id: i64, x: f32, y: f32);

    /// A player/entity left the game.
    #[signal]
    fn player_left(entity_id: i64);

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

async fn poll_loop(
    addr: String,
    client_id: u32,
    username: String,
    inbox: Inbox,
    outbox: Outbox,
) {
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
        match try_connect_and_run(host, port, client_id, &username, &inbox, &outbox).await {
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
    client_id: u32,
    username: &str,
    inbox:  &Inbox,
    outbox: &Outbox,
) -> anyhow::Result<()> {
    let peer = GamePeer::new(QuicBackend::new());
    peer.connect(host, port)?;
    tracing::info!("mmo_quic: connecting to broker {host}:{port}");

    let mut peer = peer;
    let mut connection: Option<GameConnection> = None;
    let mut stream:     Option<GameStream> = None;
    let mut register_sent = false;

    loop {
        loop {
            match peer.poll() {
                Ok(Some(event)) => handle_event(
                    &mut peer, event,
                    client_id, username,
                    &mut connection, &mut stream, &mut register_sent,
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
    client_id:   u32,
    username:    &str,
    connection:  &mut Option<GameConnection>,
    stream:      &mut Option<GameStream>,
    register_sent: &mut bool,
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

            if !*register_sent {
                let packet = encode_register_client(client_id, username);

                if packet.is_empty() {
                    tracing::error!(
                        "mmo_quic: RegisterClient packet was empty for client_id={} username={}",
                        client_id,
                        username
                    );
                } else if let Err(e) = peer.send(&conn, &st, Bytes::from(packet)) {
                    tracing::error!("mmo_quic: failed to send RegisterClient: {e}");
                } else {
                    *register_sent = true;
                    tracing::info!(
                        "mmo_quic: RegisterClient sent client_id={} username={}",
                        client_id,
                        username
                    );
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
            *register_sent = false;
        }

        GameNetworkEvent::StreamClosed(_, _) => {
            *stream     = None;
            *register_sent = false;
        }

        GameNetworkEvent::Error { inner, .. } => {
            tracing::warn!("mmo_quic: socket error: {inner}");
        }

        _ => {}
    }
}
