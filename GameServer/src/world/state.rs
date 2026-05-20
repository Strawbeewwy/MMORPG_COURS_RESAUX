use crate::net::network_event::SharedPlayerRegistry;
use crate::world::player::PlayerInfo;
use bevy::prelude::{
    Res, Resource, Time
};
use shared::protocol::{
    PlayerId, PlayerSnapshot, WorldSnapshot, ZoneId
};
use crate::net::area_of_interest::{
    is_inside_area_of_interest, DEFAULT_AREA_OF_INTEREST_RADIUS,
};
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

    pub fn snapshot_for_player(
        &self,
        zone: ZoneId,
        observer_player_id: &PlayerId,
        radius: f32,
    ) -> Option<WorldSnapshot> {
        let observer = self.players.get(observer_player_id)?;

        let players = self
            .players
            .values()
            .filter(|player| {
                player.player_id == observer.player_id
                    || is_inside_area_of_interest(observer.position, player.position, radius)
            })
            .map(|player| PlayerSnapshot {
                player_id: player.player_id.clone(),
                username: player.username.clone(),
                position: player.position,
                velocity: player.velocity,
            })
            .collect();

        Some(WorldSnapshot {
            zone,
            players,
            server_tick: self.server_tick,
        })
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