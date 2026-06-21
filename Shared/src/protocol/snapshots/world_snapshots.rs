use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::combat::ColorTeam;
use crate::enemy::EnemySnapshot;
use crate::projectile::ProjectileSnapshot;
use crate::protocol::{PlayerPublicInfo, PlayerSnapshot, ClientId};
use crate::protocol::snapshots::entity_snapshot::EntitySnapshot;
use crate::protocol::utils::utils::{read_arc_str, read_client_id, read_u16, read_u64, read_u8, write_arc_str, write_client_id, write_len_u16, write_u64, write_u8, BinaryDecode, BinaryEncode};

/// Shared zone identifier — uses `Arc<str>` instead of `String` to avoid repeated
/// heap allocations when the same zone name is cloned across many utils messages.
/// Serde serialises/deserialises `Arc<str>` as a plain JSON string transparently.

pub type ZoneId = Arc<str>;

const WORLD_UPDATE_SNAPSHOT: u8 = 0x01;
const WORLD_UPDATE_PLAYER_JOINED: u8 = 0x02;
const WORLD_UPDATE_PLAYER_LEFT: u8 = 0x03;



/// World-state update broadcast by the utils to subscribed clients.

/// World-state update broadcast by the broker to subscribed clients.
#[derive(Debug, Clone)]
pub enum WorldUpdate {
    /// Full world snapshot for initial sync or re-sync.
    Snapshot { snapshot: WorldSnapshot },
    /// A new player appeared in the zone.
    PlayerJoined { player: PlayerPublicInfo, client_id: ClientId },
    /// A player left the zone.
    PlayerLeft { player: PlayerPublicInfo, client_id: ClientId },

    // ── 5SecsSwap gameplay events ────────────────────────────────────────────

    /// The global 5-second colour swap fired.
    /// `swap_index` increments each swap — even = Red active, odd = Blue active.
    ColorSwap { swap_index: u64 },
    /// Server assigned this client a starting colour team.
    PlayerColorAssigned { client_id: ClientId, color: ColorTeam },
    /// Batch enemy state sent every tick.
    EnemiesUpdate { enemies: Vec<EnemySnapshot> },
    /// An enemy was killed; killer may be None (e.g. fell into a pit).
    EnemyDied { enemy_id: u32, killer_client_id: Option<ClientId> },
    /// Batch projectile state sent every tick.
    ProjectilesUpdate { projectiles: Vec<ProjectileSnapshot> },
    /// Score delta for a player (cumulative on the client side).
    PlayerScoreUpdate { client_id: ClientId, score: u32 },
}


/**
snapshot of the world, sent to the client
**/
#[derive(Debug, Clone, PartialEq)]
pub struct WorldSnapshot {
    pub zone: ZoneId,
    pub players: Vec<PlayerSnapshot>,
    pub entities: Vec<EntitySnapshot>,
    pub server_tick: u64,
}


impl BinaryEncode for WorldUpdate {
    fn encode_binary(&self, output: &mut Vec<u8>) -> anyhow::Result<()> {
        match self {
            WorldUpdate::Snapshot { snapshot } => {
                write_u8(output, WORLD_UPDATE_SNAPSHOT);
                snapshot.encode_binary(output)?;
            }
            WorldUpdate::PlayerJoined { player, client_id } => {
                write_u8(output, WORLD_UPDATE_PLAYER_JOINED);
                write_client_id(output, *client_id);
                player.encode_binary(output)?;
            }
            WorldUpdate::PlayerLeft { player, client_id } => {
                write_u8(output, WORLD_UPDATE_PLAYER_LEFT);
                write_client_id(output, *client_id);
                player.encode_binary(output)?;
            }
            WorldUpdate::ColorSwap { .. } => {}
            WorldUpdate::PlayerColorAssigned { .. } => {}
            WorldUpdate::EnemiesUpdate { .. } => {}
            WorldUpdate::EnemyDied { .. } => {}
            WorldUpdate::ProjectilesUpdate { .. } => {}
            WorldUpdate::PlayerScoreUpdate { .. } => {}
        }

        Ok(())
    }
}

impl BinaryDecode for WorldUpdate {
    fn decode_binary(input: &mut &[u8]) -> anyhow::Result<Self> {
        let tag = read_u8(input)?;

        match tag {
            WORLD_UPDATE_SNAPSHOT => {
                let snapshot = WorldSnapshot::decode_binary(input)?;
                Ok(WorldUpdate::Snapshot { snapshot })
            }
            WORLD_UPDATE_PLAYER_JOINED => {
                let client_id = read_client_id(input)?;
                let player = PlayerPublicInfo::decode_binary(input)?;
                Ok(WorldUpdate::PlayerJoined { player, client_id })
            }
            WORLD_UPDATE_PLAYER_LEFT => {
                let client_id = read_client_id(input)?;
                let player = PlayerPublicInfo::decode_binary(input)?;
                Ok(WorldUpdate::PlayerLeft { player, client_id })
            }
            unknown => {
                anyhow::bail!("unknown WorldUpdate tag: 0x{unknown:02x}");
            }
        }
    }
}

impl BinaryEncode for WorldSnapshot {
    fn encode_binary(&self, output: &mut Vec<u8>) -> anyhow::Result<()> {
        write_arc_str(output, &self.zone)?;
        write_u64(output, self.server_tick);

        write_len_u16(output, self.players.len(), "player count")?;
        for player in &self.players {
            player.encode_binary(output)?;
        }

        write_len_u16(output,self.entities.len(), "entity count")?;
        for entity in &self.entities {
            entity.encode_binary(output)?;
        }

        Ok(())
    }
}

impl BinaryDecode for WorldSnapshot {
    fn decode_binary(input: &mut &[u8]) -> anyhow::Result<Self> {
        let zone = read_arc_str(input)?;
        let server_tick = read_u64(input)?;
        let player_count = read_u16(input)? as usize;

        let mut players = Vec::with_capacity(player_count);

        for _ in 0..player_count {
            players.push(PlayerSnapshot::decode_binary(input)?);
        }

        let entity_count = read_u16(input)? as usize;
        let mut entities = Vec::with_capacity(entity_count);
        for _ in 0..entity_count {
            entities.push(EntitySnapshot::decode_binary(input)?);
        }

        Ok(WorldSnapshot {
            zone,
            players,
            entities,
            server_tick,
        })
    }
}