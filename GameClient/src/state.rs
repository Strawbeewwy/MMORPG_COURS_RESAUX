use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct LocalPlayerState {
    pub player_id: Option<String>,
    pub zone: Option<String>,
    pub last_movement_x: f32,
    pub last_movement_y: f32,
}