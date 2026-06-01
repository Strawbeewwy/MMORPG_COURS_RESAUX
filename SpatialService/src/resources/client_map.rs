use bevy::prelude::*;
use game_sockets::GameConnection;
use std::collections::{HashMap, HashSet};
use shared::protocol::broker::{ClientId, ShardId};

/// Tracks whether a client is idle or mid-handoff.
/// Prevents duplicate HandoffRequest messages for the same client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientTransferState {
    /// Client is stable on its current shard — handoff may be initiated.
    Stable,
    /// A HandoffRequest has been sent to `destination_shard`; awaiting HandoffAck.
    PendingHandoff { destination_shard: u32 },
}

/// Tracks each client's currently subscribed shard id and transfer state.
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
    pub shard_by_client: HashMap<ClientId, ShardId>,
    /// GameConnection (shard) → set of client_ids routed through it
    connection_clients: HashMap<GameConnection, HashSet<ClientId>>,
    /// client_id → transfer state (absent = Stable, to avoid allocating per client)
    client_states: HashMap<ClientId, ClientTransferState>,
    
}

impl ClientMap {
    /// Insert or update a client's shard, recording the shard connection for cleanup.
    pub fn insert(&mut self, client_id: ClientId, shard_id: ShardId, conn: GameConnection) {
        self.shard_by_client.insert(client_id, shard_id);
        self.connection_clients
            .entry(conn)
            .or_default()
            .insert(client_id);
    }

    /// Get the current shard for a client.
    pub fn get(&self, client_id: ClientId) -> Option<ShardId> {
        self.shard_by_client.get(&client_id).copied()
    }

    /// Remove a single client (e.g. explicit logout).
    /// Cleans up both `shard_by_client` and the reverse `connection_clients` index
    /// to prevent unbounded memory growth on long-running servers.
    pub fn remove(&mut self, client_id: ClientId) -> Option<ShardId> {
        let shard = self.shard_by_client.remove(&client_id);
        self.client_states.remove(&client_id);
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
                self.client_states.remove(&client_id);
            }
        }
    }

    // ── Transfer state ─────────────────────────────────────────────────────

    /// Returns `true` if the client is in `Stable` state (no handoff in progress).
    pub fn is_stable(&self, client_id: ClientId) -> bool {
        !matches!(
            self.client_states.get(&client_id),
            Some(ClientTransferState::PendingHandoff { .. })
        )
    }

    /// Mark a client as pending handoff. Idempotent if called twice for the same destination.
    pub fn set_state(&mut self, client_id: ClientId, state: ClientTransferState) {
        self.client_states.insert(client_id, state);
    }

    /// Clear the transfer state (called on HandoffAck or on client disconnect).
    pub fn clear_state(&mut self, client_id: ClientId) {
        self.client_states.remove(&client_id);
    }

    /// Read the transfer state for a client (absent = Stable).
    pub fn get_state(&self, client_id: ClientId) -> ClientTransferState {
        self.client_states
            .get(&client_id)
            .cloned()
            .unwrap_or(ClientTransferState::Stable)
    }
}
