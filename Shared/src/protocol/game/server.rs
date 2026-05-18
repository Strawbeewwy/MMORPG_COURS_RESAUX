use crate::protocol::game::{PlayerId, PlayerPublicInfo, WorldSnapshot};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerGameMessage {
    JoinAccepted {///when the server accepts a player connection
        player_id: PlayerId,
        player: PlayerPublicInfo,
        snapshot: WorldSnapshot,
        message: String,
    },
    JoinRejected {///when the server rejects a player connection
        reason: String,
    },
    HeartbeatAck,///server sends this to the client to confirm the heartbeat sent by the client
    InputAccepted {
        movement_x: f32,
        movement_y: f32,
    },
    WorldSnapshot {
        snapshot: WorldSnapshot,
    },
    PlayerJoined {///when a player joins the game
        player: PlayerPublicInfo,
    },
    PlayerLeft {///when a player leaves the game
        player_id: PlayerId,
    },
    ///can be used later when a server closes to confirm with the orchestrator
    Goodbye,
}