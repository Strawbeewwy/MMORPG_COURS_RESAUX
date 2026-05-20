use crate::net::network_event::SharedPlayerRegistry;
use crate::world::player::PlayerInfo;
use bevy::prelude::{Res, Resource, Time};
use shared::protocol::{PlayerId, PlayerSnapshot, WorldSnapshot, ZoneId};
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

    pub fn update_players(&mut self, delta_seconds: f32) {
        for player in self.players.values_mut() {
            player.update_movement(delta_seconds);
        }

        self.server_tick += 1;
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

pub fn update_players_registry(
    registry: Res<SharedPlayerRegistry>,
    time: Res<Time>,
) {
    let Ok(mut registry) = registry.inner.try_lock() else {
        return;
    };

    registry.update_players(time.delta_secs());
}