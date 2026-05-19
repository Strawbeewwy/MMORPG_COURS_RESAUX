use crate::world::player::PlayerInfo;
use bevy::prelude::Resource;
use shared::protocol::{PlayerSnapshot, WorldSnapshot, ZoneId, PlayerId};
use std::collections::HashMap;

#[derive(Debug, Default, Resource)]
pub struct PlayerRegistry {
    pub players: HashMap<PlayerId, PlayerInfo>,
    pub server_tick: u64,
}

impl PlayerRegistry {
    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    pub fn is_full(&self, max_players: usize) -> bool {
        self.players.len() >= max_players
    }

    pub fn snapshot(&self, zone: ZoneId) -> WorldSnapshot {
        let players = self
            .players
            .values()
            .map(|player| PlayerSnapshot {
                player_id: player.player_id.clone(),
                username: player.username.clone(),
                position: player.position,
                velocity: player.velocity,
            })
            .collect();

        WorldSnapshot {
            zone,
            players,
            server_tick: self.server_tick,
        }
    }
}