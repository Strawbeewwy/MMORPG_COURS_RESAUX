use std::collections::HashMap;

use game_sockets::{GameConnection, GamePeer, GameStream};
use crate::protocol::ShardId;

pub struct ShardHandle {
    pub peer: GamePeer,
    /// One reliable stream per connected shard connection.
    pub streams: HashMap<GameConnection, GameStream>,
    /// shard_id → GameConnection (for routing HandoffRequest to the right shard).
    pub connection_by_shard_id: HashMap<ShardId, GameConnection>,
    /// GameConnection → shard_id (for fast lookup on message receipt).
    pub shard_id_by_connection: HashMap<GameConnection, ShardId>,
}

impl ShardHandle {
    /// Register a shard's identity once it sends a ShardRegister message.
    pub fn register_shard(&mut self, conn: GameConnection, shard_id: ShardId) {
        self.connection_by_shard_id.insert(shard_id, conn);
        self.shard_id_by_connection.insert(conn, shard_id);
    }

    /// Remove a shard's registration on disconnect.
    pub fn unregister_shard(&mut self, conn: GameConnection) {
        if let Some(shard_id) = self.shard_id_by_connection.remove(&conn) {
            self.connection_by_shard_id.remove(&shard_id);
        }
        self.streams.remove(&conn);
    }

    /// Send a raw payload to a specific shard by its id.
    /// Returns `true` if the shard is connected and the send succeeded.
    pub fn send_to_shard(&self, shard_id: ShardId, payload: Vec<u8>) -> anyhow::Result<(bool)> {
        let Some(conn) = self.connection_by_shard_id.get(&shard_id) else {
            anyhow::bail!("send_to_shard: shard {} not connected", shard_id.0)
        };
        let Some(stream) = self.streams.get(conn) else {
            anyhow::bail!("send_to_shard: shard {} has no stream yet", shard_id.0)
        };
        match self.peer.send(conn, stream, payload.into()) {
            Ok(_) => Ok(true),
            Err(e) => {
                anyhow::bail!("send_to_shard: failed to send to shard {}: {e}", shard_id.0);
            }
        }
    }
}