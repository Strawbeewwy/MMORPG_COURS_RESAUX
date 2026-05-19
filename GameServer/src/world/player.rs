use shared::protocol::{NetVec2, PlayerId, PlayerPublicInfo, Username, ZoneId};
#[derive(Debug, Clone)]
pub struct PlayerInfo {
    pub player_id: PlayerId,
    pub username: Username,
    pub zone: ZoneId,
    pub position: NetVec2,
    pub velocity: NetVec2,
}

impl PlayerInfo {
    pub fn public_info(&self) -> PlayerPublicInfo {
        PlayerPublicInfo {
            player_id: self.player_id.clone(),
            username: self.username.clone(),
            zone: self.zone.clone(),
        }
    }
}