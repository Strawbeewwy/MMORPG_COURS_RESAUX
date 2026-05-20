use shared::protocol::{NetVec2, PlayerId, PlayerPublicInfo, Username, ZoneId};

pub const PLAYER_DEFAULT_MOVE_SPEED: f32 = 5.0;
#[derive(Debug, Clone)]
pub struct PlayerInfo {
    pub player_id: PlayerId,
    pub username: Username,
    pub zone: ZoneId,
    pub position: NetVec2,
    pub velocity: NetVec2,
    pub movement_speed: f32,
}

impl PlayerInfo {
    pub fn public_info(&self) -> PlayerPublicInfo {
        PlayerPublicInfo {
            player_id: self.player_id.clone(),
            username: self.username.clone(),
            zone: self.zone.clone(),
        }
    }

    pub fn update_movement(&mut self, delta_seconds: f32) {

        self.position.x += self.velocity.x * self.movement_speed * delta_seconds;
        self.position.y += self.velocity.y * self.movement_speed * delta_seconds;
    }
}