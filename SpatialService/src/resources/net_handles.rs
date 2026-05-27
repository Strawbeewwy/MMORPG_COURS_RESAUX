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

/// Outbound QUIC connection to the broker.
/// Used to send Subscribe / Unsubscribe messages.
#[derive(Resource)]
pub struct BrokerClient {
    pub peer: GamePeer,
    pub connection: Option<GameConnection>,
    /// Reliable stream used to send broker control messages.
    pub stream: Option<GameStream>,
}

impl BrokerClient {
    /// True once the handshake is complete and a stream is ready.
    pub fn is_ready(&self) -> bool {
        self.connection.is_some() && self.stream.is_some()
    }

    /// Send raw bytes over the reliable broker stream if available.
    pub fn send(&self, payload: Vec<u8>) {
        let (Some(conn), Some(stream)) = (self.connection.as_ref(), self.stream.as_ref()) else {
            tracing::warn!("BrokerClient not ready — dropping message");
            return;
        };
        if let Err(e) = self.peer.send(conn, stream, payload.into()) {
            tracing::error!("failed to send to broker: {e}");
        }
    }
}

