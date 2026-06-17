use std::collections::{HashMap, HashSet};
use bevy::prelude::*;
use std::net::{SocketAddr};
use std::time::{Duration, Instant};
pub(crate) use shared::protocol::{
    BrokerConnectionState, BrokerHandle, ShardId,
};
use shared::protocol::net_handles::shard_handle::ShardHandle;
use shared::config::DEFAULT_MAX_ENTITIES;

/// Listens for incoming QUIC connections from shards.
/// Shards connect here to push PositionUpdate messages and receive HandoffRequest.
#[derive(Resource)]
pub struct ShardListener {
    pub handle :ShardHandle,
}

impl ShardListener {
    pub fn new(handle: ShardHandle) -> Self {
        Self {
            handle,
        }
    }
}


/// Outbound QUIC connection to the broker.
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

/// UDP Connection to the orchestrator
/// used to send request to start servers
/// when we divide the quad tree
#[derive(Resource, Debug, Clone)]
pub struct OrchestratorClient {
    pub command_addr: SocketAddr,
    pub last_request_by_shard: HashMap<ShardId, Instant>,
    pub request_cooldown: Duration,
    pub max_entities_per_shard: usize,
    pub split_parent_candidates: HashSet<ShardId>,
    pub last_stop_request_by_shard: HashMap<ShardId, Instant>,
    pub stop_request_cooldown: Duration,
}

impl OrchestratorClient {
    pub fn new(addr : SocketAddr) -> Self {
        Self {
            command_addr: addr,
            last_request_by_shard: HashMap::default(),
            request_cooldown: Duration::from_secs(10),
            max_entities_per_shard: DEFAULT_MAX_ENTITIES as usize,
            split_parent_candidates: HashSet::default(),
            last_stop_request_by_shard: HashMap::default(),
            stop_request_cooldown: Duration::from_secs(10),
        }
    }

    pub fn should_request_server(&mut self, shard_id: ShardId, current_count: usize) -> bool {
        if current_count <= self.max_entities_per_shard {
            return false;
        }

        let now = Instant::now();

        if let Some(last_request) = self.last_request_by_shard.get(&shard_id) {
            if now.duration_since(*last_request) < self.request_cooldown {
                return false;
            }
        }

        self.last_request_by_shard.insert(shard_id, now);
        true
    }

    pub fn mark_split_parent_candidate(&mut self, shard_id: ShardId) {
        self.split_parent_candidates.insert(shard_id);
    }

    pub fn clear_split_parent_candidate(&mut self, shard_id: ShardId) {
        self.split_parent_candidates.remove(&shard_id);
    }

    pub fn is_split_parent_candidate(&self, shard_id: ShardId) -> bool {
        self.split_parent_candidates.contains(&shard_id)
    }

    pub fn should_request_stop_shard(&mut self, shard_id: ShardId) -> bool {
        let now = Instant::now();

        if let Some(last_request) = self.last_stop_request_by_shard.get(&shard_id) {
            if now.duration_since(*last_request) < self.stop_request_cooldown {
                return false;
            }
        }

        self.last_stop_request_by_shard.insert(shard_id, now);
        true
    }
}
