/// Local Bevy message types for the spatial service.
/// These wrap or mirror the shared wire protocol structs,
/// keeping the `shared` crate free of any Bevy dependency.
use bevy::prelude::*;
use game_sockets::GameConnection;
use shared::protocol::{ClientId, ShardId};

/// Bevy message produced by `poll_shard_events` from an incoming PositionUpdate wire packet.
///
/// # Why f64 when the wire protocol uses f32?
/// The wire format (tag `0x10`) transmits positions as `f32` to minimise bandwidth.
/// Internally we widen to `f64` to avoid precision loss near large world boundaries
/// (e.g. ±1000 world units). The cast `f32 → f64` via `f64::from` is lossless;
/// the reverse `f64 → f32` cast in `handle_subscriptions` is intentional and documented there.
#[derive(Message, Debug, Clone, Copy)]
pub struct PositionUpdateMsg {
    pub client_id: ClientId,
    /// The direct QUIC connection of the shard that sent this update.
    /// `Some` when received via the shard listener (direct shard → SpatialService path).
    /// `None` when relayed through the utils.
    pub shard_connection: Option<GameConnection>,
    pub x: f64,
    pub y: f64,
}

/// Maximum number of distinct shards that can border a single point.
/// Bounded by the QuadTree structure: at most 4 leaf neighbours at any boundary.
pub const MAX_CROSSING_SHARDS: usize = 4;

/// Bevy message emitted when a client is near a shard boundary.
/// Uses a fixed-size inline array to avoid heap allocation per alert.
/// Stub for Part 3 (HandoffRequest).
#[derive(Message, Debug, Clone, Copy)]
pub struct CrossingAlertMsg {
    pub client_id: ClientId,
    /// Inline shard ids — valid entries are `shards[..shard_count]`.
    pub shards: [ShardId; MAX_CROSSING_SHARDS],
    pub shard_count: usize,
}

impl CrossingAlertMsg {
    /// Build from a slice (truncates beyond MAX_CROSSING_SHARDS with a warning).
    pub fn from_slice(client_id: ClientId, ids: &[ShardId]) -> Self {
        if ids.len() > MAX_CROSSING_SHARDS {
            tracing::warn!(
                "CrossingAlertMsg: client {} has {} crossing shards but only {} can be tracked — \
                 excess shards silently dropped (QuadTree depth may be too shallow)",
                client_id.0,
                ids.len(),
                MAX_CROSSING_SHARDS,
            );
        }
        let mut shards = [ShardId::default(); MAX_CROSSING_SHARDS];

        let shard_count = ids.len().min(MAX_CROSSING_SHARDS);
        let valid_ids = &ids[..shard_count];

        shards.iter_mut()
            .zip(valid_ids.iter())
            .for_each(|(slot, &id)| *slot = id);

        Self {
            client_id,
            shards,
            shard_count
        }
    }

    /// Iterate over the valid shard ids.
    pub fn iter_shards(&self) -> &[ShardId] {
        &self.shards[..self.shard_count]
    }
}

/// Bevy message emitted by `handle_crossing_alerts` when a client should be handed off
/// to a neighbouring shard. Consumed by `handle_handoff_requests`.
#[derive(Message, Debug, Clone, Copy)]
pub struct HandoffRequestMsg {
    pub client_id: ClientId,
    /// The shard the client is currently subscribed to.
    pub from_shard: ShardId,
    /// The shard the client is crossing into.
    pub to_shard: ShardId,
}
