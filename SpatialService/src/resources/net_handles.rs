use bevy::prelude::*;
use game_sockets::{GameConnection, GamePeer, GameStream};
use std::collections::HashMap;

/// Listens for incoming QUIC connections from shards.
/// Shards connect here to push PositionUpdate messages.
#[derive(Resource)]
pub struct ShardListener {
    pub peer: GamePeer,
    /// One reliable stream per connected shard connection.
    pub streams: HashMap<GameConnection, GameStream>,
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
}

impl BrokerClient {
    pub fn new(peer: GamePeer) -> Self {
        Self {
            peer,
            connection: None,
            stream: None,
            state: BrokerConnectionState::Connecting,
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

    /// Reset state for a reconnect attempt.
    pub fn reset_for_reconnect(&mut self) {
        self.connection = None;
        self.stream = None;
        self.state = BrokerConnectionState::Connecting;
    }
}

