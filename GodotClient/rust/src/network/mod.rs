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

/// Decode a single Broker message from raw bytes.
/// Returns None for unknown / malformed messages (logged as warnings).
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
        NetworkMessage::Broadcast {payload,payload_len} =>{
            Some(IncomingEvent::Broadcast {payload})
        }
        _ => {
            tracing::debug!("mmo_client: ignoring message {:?}", message);
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
                    // Forward the raw WorldUpdate payload to GDScript.
                    let bytes: PackedByteArray = payload.iter().copied().collect();
                    self.base_mut().emit_signal("broadcast_received", &[bytes.to_variant()]);
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

    // ── GDScript-callable functions ────────────────────────────────────────────

    /// Send the local player's movement direction to the Broker.
    /// x / y are normalised direction values in [-1, 1].
    #[func]
    fn send_movement(&self, x: f32, y: f32) {
        let pkt = encode_client_input(self.client_id, x, y);
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
    let mut connection: Option<GameConnection> = None;
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
    connection:  &mut Option<GameConnection>,
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
