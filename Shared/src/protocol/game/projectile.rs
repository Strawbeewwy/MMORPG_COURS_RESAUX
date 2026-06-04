/// Projectile protocol types.
///
/// Projectiles are spawned by the GameServer when a player fires and are
/// broadcast as `WorldUpdate::ProjectilesUpdate` every tick.
/// Dead projectiles have `alive = false` — clients remove them.
use serde::{Deserialize, Serialize};
use crate::protocol::game::combat::ColorTeam;
use crate::protocol::broker::ClientId;
use crate::protocol::NetVec2;

/// Unique identifier for a server-side projectile.
pub type ProjectileId = u32;

/// Per-projectile state snapshot broadcast to clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectileSnapshot {
    pub id:              ProjectileId,
    pub owner_client_id: ClientId,
    pub position:        NetVec2,
    /// Normalised direction * 1000 (uses NetVec2 for compactness).
    pub direction:       NetVec2,
    pub color:           ColorTeam,
    pub alive:           bool,
}

