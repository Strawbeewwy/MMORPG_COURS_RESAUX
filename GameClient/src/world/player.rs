use crate::render::renderable::{ClientRenderEntity, RenderedEntityType};
use bevy::prelude::*;
use shared::protocol::{PlayerId, PlayerSnapshot};

const PLAYER_SIZE: f32 = 32.0;
const LOCAL_PLAYER_COLOR: Color = Color::srgb(0.2, 0.8, 1.0);
const REMOTE_PLAYER_COLOR: Color = Color::srgb(1.0, 0.25, 0.25);
const PLAYER_Z_INDEX: f32 = 6.0;

#[derive(Debug, Clone)]
pub struct ClientPlayer {
    pub snapshot: PlayerSnapshot,
    pub is_local_player: bool,
}

impl ClientPlayer {
    pub fn new(snapshot: PlayerSnapshot, local_player_id: Option<&PlayerId>) -> Self {
        let is_local_player = local_player_id
            .is_some_and(|local_player_id| local_player_id == &snapshot.player_id);

        Self {
            snapshot,
            is_local_player,
        }
    }

    pub fn player_id(&self) -> &PlayerId {
        &self.snapshot.player_id
    }

    pub fn create_render_entity(&self) -> ClientRenderEntity {
        ClientRenderEntity {
            entity_id: self.snapshot.player_id.clone(),
            entity_type: RenderedEntityType::Player,
            position: self.snapshot.position,
            color: self.color(),
            size: self.size(),
            z_index: self.z_index(),
        }
    }

    pub fn color(&self) -> Color {
        if self.is_local_player {
            LOCAL_PLAYER_COLOR
        } else {
            REMOTE_PLAYER_COLOR
        }
    }

    pub fn size(&self) -> Vec2 {
        Vec2::splat(PLAYER_SIZE)
    }

    pub fn z_index(&self) -> f32 {
        PLAYER_Z_INDEX
    }
}