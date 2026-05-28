/// CrossingAlert is internal (Bevy event) — triggers handoff logic in Part 3.

/// Internal Bevy event — emitted when a client is near a shard boundary.
/// Consumed by the crossing system; will trigger HandoffRequest in Part 3.
#[derive(Debug, Clone)]
pub struct CrossingAlert {
    pub client_id: u32,
    /// All distinct shard ids covering the margin area around the client position.
    pub shards: Vec<u32>,
}

