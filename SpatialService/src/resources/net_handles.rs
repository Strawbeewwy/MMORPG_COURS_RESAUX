use bevy::prelude::*;
use shared::game_sockets::{GameConnection, GameNetworkEvent, GamePeer, GameSocketError, GameStream};
use std::collections::HashMap;
use std::time::{Duration, Instant};
pub(crate) use shared::protocol::{BrokerConnectionState, BrokerHandle, ShardId};
use shared::protocol::net_handles::shard_handle::ShardHandle;

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

