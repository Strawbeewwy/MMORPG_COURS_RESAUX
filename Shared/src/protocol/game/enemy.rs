/// Enemy protocol types.
///
/// Enemies are purely server-authoritative.  The GameServer maintains the full
/// simulation (AI, HP, despawn) and sends `EnemySnapshot` batches to clients
/// inside `WorldUpdate::EnemiesUpdate` every server tick.
use serde::{Deserialize, Serialize};
use crate::protocol::game::combat::ColorTeam;
use crate::protocol::NetVec2;

/// Unique identifier for a server-side enemy.
pub type EnemyId = u32;

/// Per-enemy state snapshot broadcast to clients each tick.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemySnapshot {
    pub id:       EnemyId,
    pub position: NetVec2,
    pub color:    ColorTeam,
    /// 0 = dead / being removed (clients should despawn the visual).
    pub hp:       u8,
}

