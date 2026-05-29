use bevy::prelude::*;
use game_sockets::GameConnection;
use std::collections::{HashMap, HashSet};

/// Tracks each client's currently subscribed shard id.
/// Also maps shard connections to their client ids for bulk cleanup on disconnect.
///
/// # Note on `GameConnection` as map key
/// `GameConnection` is used as a `HashMap` key via its `Hash`/`Eq` impls provided
/// by `game_sockets`. If the underlying library ever changes its identity semantics
/// (e.g. connection reuse), this map must be audited. Prefer a stable `u64` id
/// if the API exposes one in the future.
#[derive(Resource, Default, Debug)]
pub struct ClientMap {
    /// client_id → current shard_id
    pub shard_by_client: HashMap<u32, u32>,
    /// GameConnection (shard) → set of client_ids routed through it
    connection_clients: HashMap<GameConnection, HashSet<u32>>,
}

impl ClientMap {
    /// Insert or update a client's shard, recording the shard connection for cleanup.
    pub fn insert(&mut self, client_id: u32, shard_id: u32, conn: GameConnection) {
        self.shard_by_client.insert(client_id, shard_id);
        self.connection_clients
            .entry(conn)
            .or_default()
            .insert(client_id);
    }

    /// Get the current shard for a client.
    pub fn get(&self, client_id: u32) -> Option<u32> {
        self.shard_by_client.get(&client_id).copied()
    }

    /// Remove a single client (e.g. explicit logout).
    /// Cleans up both `shard_by_client` and the reverse `connection_clients` index
    /// to prevent unbounded memory growth on long-running servers.
    pub fn remove(&mut self, client_id: u32) -> Option<u32> {
        let shard = self.shard_by_client.remove(&client_id);
        // Remove client_id from every connection set and prune empty entries.
        self.connection_clients.values_mut().for_each(|clients| {
            clients.remove(&client_id);
        });
        self.connection_clients.retain(|_, clients| !clients.is_empty());
        shard
    }

    /// Remove all clients associated with a disconnected shard connection.
    /// Prevents unbounded memory growth on long-running servers.
    pub fn remove_by_connection(&mut self, conn: GameConnection) {
        if let Some(clients) = self.connection_clients.remove(&conn) {
            for client_id in clients {
                self.shard_by_client.remove(&client_id);
            }
        }
    }
}
