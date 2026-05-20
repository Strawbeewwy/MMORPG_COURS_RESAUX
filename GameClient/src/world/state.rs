use bevy::prelude::*;
use shared::protocol::PlayerSnapshot;
use std::collections::HashMap;

#[derive(Resource, Default)]
pub struct LocalWorldState {
    pub player_id: Option<String>,
    pub zone: Option<String>,
    pub last_movement_x: f32,
    pub last_movement_y: f32,
    pub players: HashMap<String, PlayerSnapshot>,
}