use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::protocol::{PlayerPublicInfo, PlayerSnapshot, ClientId};

/// Shared zone identifier — uses `Arc<str>` instead of `String` to avoid repeated
/// heap allocations when the same zone name is cloned across many utils messages.
/// Serde serialises/deserialises `Arc<str>` as a plain JSON string transparently.

pub type ZoneId = Arc<str>;



/// World-state update broadcast by the utils to subscribed clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorldUpdate {
    /// Full world snapshot for initial sync or re-sync.
    Snapshot { snapshot: WorldSnapshot },
    /// A new player appeared in the zone.
    PlayerJoined { player: PlayerPublicInfo, client_id: ClientId },
    /// A player left the zone.
    PlayerLeft { player: PlayerPublicInfo , client_id: ClientId},
}



/**
snapshot of the world, sent to the client
**/
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub zone: ZoneId,
    pub players: Vec<PlayerSnapshot>,
    pub server_tick: u64,
}