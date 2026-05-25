use crate::protocol::game::{PlayerId, PlayerPublicInfo, WorldSnapshot};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorldUpdate {
    Snapshot {
        snapshot: WorldSnapshot,
    },
    PlayerJoined {
        player: PlayerPublicInfo,
    },
    PlayerLeft {
        player_id: PlayerId,
    },
}