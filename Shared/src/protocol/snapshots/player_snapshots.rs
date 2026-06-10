use serde::{Deserialize, Serialize};
use crate::protocol::{ClientId, NetVec2, Username};
use crate::protocol::game::PlayerId;
use crate::protocol::utils::utils::{read_client_id, read_net_vec2, read_player_id, read_username, write_client_id, write_net_vec2, write_player_id, write_username, BinaryDecode, BinaryEncode};

/**
snapshot of a player, sent to the client
**/
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerSnapshot {
    pub client_id: ClientId,
    pub player_id: PlayerId,
    pub username: Username,
    pub position: NetVec2,
    pub velocity: NetVec2,
}



impl BinaryEncode for PlayerSnapshot {
    fn encode_binary(&self, output: &mut Vec<u8>) -> anyhow::Result<()> {
        write_client_id(output, self.client_id);
        write_player_id(output, self.player_id);
        write_username(output, &self.username)?;
        write_net_vec2(output, self.position);
        write_net_vec2(output, self.velocity);

        Ok(())
    }
}

impl BinaryDecode for PlayerSnapshot {
    fn decode_binary(input: &mut &[u8]) -> anyhow::Result<Self> {
        let client_id = read_client_id(input)?;
        let player_id = read_player_id(input)?;
        let username = read_username(input)?;
        let position = read_net_vec2(input)?;
        let velocity = read_net_vec2(input)?;

        Ok(PlayerSnapshot {
            client_id,
            player_id,
            username,
            position,
            velocity,
        })
    }
}

