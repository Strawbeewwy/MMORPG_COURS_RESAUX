use std::sync::Arc;
use crate::net::network_event::SharedPlayerRegistry;
use bevy::prelude::*;
use shared::protocol::{NetVec2, WorldSnapshot, ZoneId, PlayerSpawnInfo, EntityId, Username};
use crate::net::area_of_interest::{
    is_inside_area_of_interest, DEFAULT_AREA_OF_INTEREST_RADIUS,
};

use bevy::platform::collections::HashMap;
use uuid::Uuid;
use shared::protocol::broker::ClientId;
use shared::protocol::game::EntityType;
use shared::protocol::game::player::{
    Player, PlayerId, PLAYER_DEFAULT_MOVE_SPEED, PlayerSnapshot, PlayerPublicInfo,
};
use shared::protocol::transport::codec;
use crate::config::ServerConfig;
use crate::world::combat::PlayerCombatRegistry;

#[derive(Debug, Default, Resource)]
pub struct PlayerRegistry {
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
            /*
                we can add more stuff the players need updated on
                like health, inventory, etc.
             */
        }

    }


    pub fn register_client(&mut self,client_id: ClientId, player_id: PlayerId) {

        self.client_player.insert(client_id, player_id);
    }

    pub fn register_player(&mut self, player: Player, client_id: ClientId) {
        self.player_client.insert(player.player_id, client_id);
    }

    pub fn remove_player(&mut self, player: &Player) {
        self.player_client.remove(&player.player_id);
    }


    pub fn remove_client(&mut self, client_id: &ClientId) {
        self.client_player.remove(client_id);
    }

    pub fn snapshot(&self, zone: ZoneId) -> WorldSnapshot {
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

        WorldSnapshot {
            zone,
            players,
            server_tick: 0,
        }
    }

    pub fn snapshot_for_player(
        &self,
        zone: ZoneId,
        observer_player: &ClientId,
        radius: f32,
    ) -> Option<WorldSnapshot> {
        // let observer = self.client_player.get(observer_player)?;
        //
        // let players = self
        //     .client_player
        //     .values()
        //     .filter(|player| {
        //         player == observer
        //             || is_inside_area_of_interest(observer.position, player.position, radius)
        //     })
        //     .map(|player| PlayerSnapshot {
        //         client_id,
        //         player_id:player.player_id,
        //         username: player.username,
        //         position: player.position,
        //         velocity: player.velocity,
        //     })
        //     .collect();
        //
        // Some(WorldSnapshot {
        //     zone,
        //     players,
        //     server_tick: 0,
        // })
        None
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

/// Bevy system: register every known client_id in the combat registry.
/// Runs after poll_broker_events so newly registered clients are picked up.
pub fn sync_combat_registry(
    registry: Res<SharedPlayerRegistry>,
    mut combat_reg: ResMut<crate::world::combat::PlayerCombatRegistry>,
) {
    let Ok(reg) = registry.inner.try_lock() else { return };
    for client_id in reg.client_player.keys() {
        if !combat_reg.states.contains_key(client_id) {
            combat_reg.register(*client_id);
            tracing::info!("combat registry: registered new client_id={}", client_id.0);
        }
    }
}

pub fn handle_add_client_to_shard(
    config: &ServerConfig,
    registry: &SharedPlayerRegistry,
    client_id: ClientId,
    payload: &[u8],
) {
    let spawn_info = match codec::decode::<PlayerSpawnInfo>(payload) {
        Ok(spawn_info) => spawn_info,
        Err(error) => {
            tracing::warn!(
                "failed to decode PlayerSpawnInfo for client {}: {error:#}",
                client_id.0
            );
            return;
        }
    };

    let Ok(mut registry) = registry.inner.try_lock() else {
        tracing::warn!("could not lock player registry for AddClientToShard");
        return;
    };

    let player = Player {
        player_id: Default::default(),
        username: spawn_info.username.clone(),
        zone: spawn_info.zone.clone(),
        position: NetVec2::ZERO,
        velocity: NetVec2::ZERO,
        movement_speed: PLAYER_DEFAULT_MOVE_SPEED,
    };

}

pub fn handle_register_client(
    config: &ServerConfig,
    registry: &SharedPlayerRegistry,
    client_id: ClientId,
    username: Username,
) {
    let Ok(mut registry) = registry.inner.try_lock() else {
        tracing::warn!("could not lock player registry for shard world snapshot publish");
        return;
    };
    let player = Player::new(
        Uuid::new_v4().as_u128(),
        username,
        config.zone.clone(),
    );

    registry.register_client(client_id, player.player_id);
}