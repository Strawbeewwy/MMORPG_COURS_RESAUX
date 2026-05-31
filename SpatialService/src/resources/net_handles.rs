use bevy::prelude::*;
use game_sockets::{GameConnection, GamePeer, GameStream};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Listens for incoming QUIC connections from shards.
/// Shards connect here to push PositionUpdate messages and receive HandoffRequest.
#[derive(Resource)]
pub struct ShardListener {
    pub peer: GamePeer,
    /// One reliable stream per connected shard connection.
    pub streams: HashMap<GameConnection, GameStream>,
    /// shard_id → GameConnection (for routing HandoffRequest to the right shard).
    pub connection_by_shard_id: HashMap<u32, GameConnection>,
    /// GameConnection → shard_id (for fast lookup on message receipt).
    pub shard_id_by_connection: HashMap<GameConnection, u32>,
}

impl ShardListener {
    /// Register a shard's identity once it sends a ShardRegister message.
    pub fn register_shard(&mut self, conn: GameConnection, shard_id: u32) {
        self.connection_by_shard_id.insert(shard_id, conn);
        self.shard_id_by_connection.insert(conn, shard_id);
        tracing::info!("shard {} registered on connection {}", shard_id, conn.connection_id);
    }

    /// Remove a shard's registration on disconnect.
    pub fn unregister_shard(&mut self, conn: GameConnection) {
        if let Some(shard_id) = self.shard_id_by_connection.remove(&conn) {
            self.connection_by_shard_id.remove(&shard_id);
            tracing::info!("shard {} unregistered (connection {} closed)", shard_id, conn.connection_id);
        }
        self.streams.remove(&conn);
    }

    /// Send a raw payload to a specific shard by its id.
    /// Returns `true` if the shard is connected and the send succeeded.
    pub fn send_to_shard(&self, shard_id: u32, payload: Vec<u8>) -> bool {
        let Some(conn) = self.connection_by_shard_id.get(&shard_id) else {
            tracing::warn!("send_to_shard: shard {} not connected", shard_id);
            return false;
        };
        let Some(stream) = self.streams.get(conn) else {
            tracing::warn!("send_to_shard: shard {} has no stream yet", shard_id);
            return false;
        };
        match self.peer.send(conn, stream, payload.into()) {
            Ok(_) => true,
            Err(e) => {
                tracing::error!("send_to_shard: failed to send to shard {}: {e}", shard_id);
                false
            }
        }
    }
}

/// Tracks the lifecycle of the outbound broker connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrokerConnectionState {
    /// Initial connect attempt in progress (no Connected event yet).
    Connecting,
    /// QUIC handshake done, waiting for the reliable stream to be created.
    Connected,
    /// Stream ready — messages can be sent.
    Ready,
    /// Connection was lost; reconnect should be attempted.
    Disconnected,
}

/// Outbound QUIC connection to the broker.
/// Used to send Subscribe / Unsubscribe messages.
#[derive(Resource)]
pub struct BrokerClient {
    pub peer: GamePeer,
    pub connection: Option<GameConnection>,
    /// Reliable stream used to send broker control messages.
    pub stream: Option<GameStream>,
    /// Explicit connection state — never ambiguous between "not yet" and "lost".
    pub state: BrokerConnectionState,
    /// Number of consecutive failed reconnect attempts (used for exponential backoff).
    pub reconnect_attempt: u32,
    /// Earliest wall-clock time at which the next reconnect attempt may be made.
    pub reconnect_after: Option<Instant>,
}

impl BrokerClient {
    pub fn new(peer: GamePeer) -> Self {
        Self {
            peer,
            connection: None,
            stream: None,
            state: BrokerConnectionState::Connecting,
            reconnect_attempt: 0,
            reconnect_after: None,
        }
    }

    /// Construct with an explicit initial state (use `Disconnected` when the
    /// startup connection attempt fails so `reconnect_broker_if_needed` retries).
    pub fn with_state(peer: GamePeer, state: BrokerConnectionState) -> Self {
        let reconnect_after = if state == BrokerConnectionState::Disconnected {
            Some(Instant::now() + Duration::from_secs(1))
        } else {
            None
        };
        Self {
            peer,
            connection: None,
            stream: None,
            state,
            reconnect_attempt: if reconnect_after.is_some() { 1 } else { 0 },
            reconnect_after,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.state == BrokerConnectionState::Ready
    }

    /// Send raw bytes over the reliable broker stream if ready.
    pub fn send(&self, payload: Vec<u8>) {
        let (Some(conn), Some(stream)) = (self.connection.as_ref(), self.stream.as_ref()) else {
            tracing::warn!("BrokerClient not ready (state={:?}) — dropping message", self.state);
            return;
        };
        if let Err(e) = self.peer.send(conn, stream, payload.into()) {
            tracing::error!("failed to send to broker: {e}");
        }
    }

    /// Reset state for a reconnect attempt (does not touch backoff counters).
    pub fn reset_for_reconnect(&mut self) {
        self.connection = None;
        self.stream = None;
        self.state = BrokerConnectionState::Connecting;
    }

    /// Reset backoff on successful connection establishment.
    pub fn reset_backoff(&mut self) {
        self.reconnect_attempt = 0;
        self.reconnect_after = None;
    }
}

