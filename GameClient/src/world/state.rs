use bevy::prelude::*;
use crate::render::renderable::ClientRenderEntity;
use crate::world::player::ClientPlayer;
use std::collections::HashMap;
use shared::protocol::{EntityId, PlayerId, PlayerSnapshot, ZoneId};


#[derive(Resource, Default)]
pub struct LocalWorldState {
    pub player_id: Option<PlayerId>,
    pub zone: Option<ZoneId>,
    pub last_movement_x: f32,
    pub last_movement_y: f32,
    pub players: HashMap<PlayerId, ClientPlayer>,
    pub render_entities: HashMap<EntityId, ClientRenderEntity>,
}


impl LocalWorldState {
    pub fn rebuild_render_entities(&mut self) {
        self.render_entities.clear();

        for player in self.players.values() {
            let render_entity = player.create_render_entity();

            self.render_entities
                .insert(render_entity.entity_id.clone(), render_entity);
        }
    }

    pub fn set_players_from_snapshot(&mut self, player_snapshots: Vec<PlayerSnapshot>) {
        self.players = player_snapshots
            .into_iter()
            .map(|snapshot| {
                let player = ClientPlayer::new(snapshot, self.player_id.as_ref());
                (player.snapshot.player_id.clone(), player)
            })
            .collect();

        self.rebuild_render_entities();
    }

}