/// Local Bevy message types for the spatial service.
/// These wrap or mirror the shared wire protocol structs,
/// keeping the `shared` crate free of any Bevy dependency.
use bevy::prelude::*;

/// Bevy message produced by `poll_shard_events` from an incoming PositionUpdate wire packet.
#[derive(Message, Debug, Clone, Copy)]
pub struct PositionUpdateMsg {
    pub client_id: u32,
    pub x: f32,
    pub y: f32,
}

/// Bevy message emitted when a client is near a shard boundary.
/// Stub for Part 3 (HandoffRequest).
#[derive(Message, Debug, Clone)]
pub struct CrossingAlertMsg {
    pub client_id: u32,
    /// All distinct shard ids that overlap the crossing margin around the client.
    pub shards: Vec<u32>,
}

