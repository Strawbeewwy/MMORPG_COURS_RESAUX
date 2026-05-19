use crate::net::network_event::{GameplayPeer, SharedPlayerRegistry};
use shared::game_sockets::GameConnection;
use shared::protocol::{NetVec2, ServerGameMessage};

pub fn handle_player_input(
    gameplay_peer: &GameplayPeer,
    registry: &SharedPlayerRegistry,
    connection: GameConnection,
    movement_x: f32,
    movement_y: f32,
) -> ServerGameMessage {
    let Some(player_id) = gameplay_peer.connection_players.get(&connection) else {
        return ServerGameMessage::JoinRejected {
            reason: "not_joined".to_string(),
        };
    };

    let Ok(mut registry) = registry.inner.try_lock() else {
        return ServerGameMessage::JoinRejected {
            reason: "server_busy".to_string(),
        };
    };

    let Some(player) = registry.players.get_mut(player_id) else {
        return ServerGameMessage::JoinRejected {
            reason: "not_joined".to_string(),
        };
    };

    player.velocity = NetVec2::new(movement_x, movement_y);

    ServerGameMessage::InputAccepted {
        movement_x,
        movement_y,
    }
}