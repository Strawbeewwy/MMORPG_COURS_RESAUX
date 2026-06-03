use std::sync::Arc;
use crate::net::network_event::SharedPlayerRegistry;
use bevy::prelude::*;
use shared::protocol::{NetVec2, WorldSnapshot, ZoneId, PlayerSpawnInfo, EntityId, Username};
use crate::net::area_of_interest::{
    is_inside_area_of_interest, DEFAULT_AREA_OF_INTEREST_RADIUS,
};

use bevy::platform::collections::HashMap;
use uuid::Uuid;
use shared::protocol::ClientId;
use shared::protocol::game::EntityType;
use shared::protocol::game::player::{
    Player, PlayerId, PLAYER_DEFAULT_MOVE_SPEED, PlayerSnapshot, PlayerPublicInfo,
};
use shared::protocol::transport::codec;
use crate::config::ServerConfig;

#[derive(Debug, Default, Resource)]
pub struct EntityRegistry {
    // player id -> player
    // used to perform actions on players
    pub players: HashMap<PlayerId, Player>,
    // player -> client
    // used for shard-to-client communication
    pub player_client: HashMap<PlayerId, ClientId>,
    // client -> player
    // used for client-to-shard communication
    pub client_player: HashMap<ClientId, PlayerId>,
    // entity -> type
    pub entity_type: HashMap<EntityId, EntityType>,

}

impl EntityRegistry {
    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    pub fn is_full(&self, max_players: usize) -> bool {
        self.players.len() >= max_players
    }

    pub fn update_players(&mut self, delta_seconds: f32) {
        for player in self.players.values_mut() {
            player.update_movement(delta_seconds);
            /*
                we can add more stuff the players need updated on
                like health, inventory, etc.
             */
        }

    }

    pub fn register_client(&mut self,client_id: ClientId, player_id: PlayerId) {

        self.client_player.insert(client_id, player_id);
        self.player_client.insert(player_id, client_id);
    }

    pub fn register_player(&mut self, player: Player) {
        self.players.insert(player.player_id, player);
    }

    pub fn remove_player(&mut self, player: &Player) {
        self.players.remove(&player.player_id);
    }


    pub fn remove_client(&mut self, client_id: &ClientId) {
        self.player_client.remove(self.client_player.get(client_id).unwrap());
        self.client_player.remove(client_id);
    }

    pub fn generate_player_snapshot(&self)-> Vec<PlayerSnapshot> {
        let players = self
            .players
            .values()
            .map(|player| PlayerSnapshot {
                client_id: self.player_client.get(&player.player_id).unwrap().clone(),
                player_id: player.player_id.clone(),
                username: player.username.clone(),
                position: player.position.clone(),
                velocity: player.velocity.clone(),
            })
            .collect();

      players
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

pub fn handle_register_client(
    config: &ServerConfig,
    registry: &SharedPlayerRegistry,
    client_id: ClientId,
    username: Username,
){
    let Ok(mut registry) = registry.inner.try_lock() else {
        tracing::warn!("could not lock player registry for shard world snapshot publish");
        return;
    };
    let player = Player {
        player_id: Uuid::new_v4().as_u128(),
        username,
        zone : config.zone.clone(),
        position: NetVec2::ZERO,
        velocity: NetVec2::ZERO,
        movement_speed: PLAYER_DEFAULT_MOVE_SPEED,
    };


    registry.register_client(client_id,player.player_id);
    registry.register_player(player);
}


