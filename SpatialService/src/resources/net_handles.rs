use bevy::prelude::*;
use game_sockets::{GameConnection, GameNetworkEvent, GamePeer, GameSocketError, GameStream};
use std::collections::HashMap;
use std::time::{Duration, Instant};
pub(crate) use shared::protocol::{BrokerConnectionState, BrokerHandle, ShardId};

/// Listens for incoming QUIC connections from shards.
/// Shards connect here to push PositionUpdate messages and receive HandoffRequest.
#[derive(Resource)]
pub struct ShardListener {
    pub peer: GamePeer,
    /// One reliable stream per connected shard connection.
    pub streams: HashMap<GameConnection, GameStream>,
    /// shard_id → GameConnection (for routing HandoffRequest to the right shard).
    pub connection_by_shard_id: HashMap<ShardId, GameConnection>,
    /// GameConnection → shard_id (for fast lookup on message receipt).
    pub shard_id_by_connection: HashMap<GameConnection, ShardId>,
}

impl ShardListener {
    /// Register a shard's identity once it sends a ShardRegister message.
    pub fn register_shard(&mut self, conn: GameConnection, shard_id: ShardId) {
        self.connection_by_shard_id.insert(shard_id, conn);
        self.shard_id_by_connection.insert(conn, shard_id);
        tracing::info!("shard {} registered on connection {}", shard_id.0, conn.connection_id);
    }

    /// Remove a shard's registration on disconnect.
    pub fn unregister_shard(&mut self, conn: GameConnection) {
        if let Some(shard_id) = self.shard_id_by_connection.remove(&conn) {
            self.connection_by_shard_id.remove(&shard_id);
            tracing::info!("shard {} unregistered (connection {} closed)", shard_id.0, conn.connection_id);
        }
        self.streams.remove(&conn);
    }

    /// Send a raw payload to a specific shard by its id.
    /// Returns `true` if the shard is connected and the send succeeded.
    pub fn send_to_shard(&self, shard_id: ShardId, payload: Vec<u8>) -> bool {
        let Some(conn) = self.connection_by_shard_id.get(&shard_id) else {
            tracing::warn!("send_to_shard: shard {} not connected", shard_id.0);
            return false;
        };
        let Some(stream) = self.streams.get(conn) else {
            tracing::warn!("send_to_shard: shard {} has no stream yet", shard_id.0);
            return false;
        };
        match self.peer.send(conn, stream, payload.into()) {
            Ok(_) => true,
            Err(e) => {
                tracing::error!("send_to_shard: failed to send to shard {}: {e}", shard_id.0);
                false
            }
        }
    }
}


/// Outbound QUIC connection to the utils.
/// Used to send Subscribe / Unsubscribe messages.
/// We wrap the BrokerHandle in a resource to manage the connection state and provide
/// a convenient interface for sending messages.
/// Also to prevent any wrong access to the handle.
#[derive(Resource)]
pub struct BrokerClient {
    pub handle: BrokerHandle,
}

impl BrokerClient {
    pub fn new(handle: BrokerHandle) -> Self {
        Self {
            handle,
        }
    }
}

