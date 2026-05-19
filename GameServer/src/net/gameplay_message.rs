use crate::config::ServerConfig;
use crate::net::input::handle_player_input;
use crate::net::login::{handle_join_game, handle_leave_game};
use crate::net::network_event::{GameplayPeer, SharedPlayerRegistry};
use shared::game_sockets::GameConnection;
use shared::protocol::{ClientGameMessage, ServerGameMessage};

pub fn handle_client_message(
    config: &ServerConfig,
    registry: &SharedPlayerRegistry,
    gameplay_peer: &mut GameplayPeer,
    connection: GameConnection,
    message: ClientGameMessage,
) -> ServerGameMessage {
    match message {
        ClientGameMessage::JoinGame {
            protocol_version,
            session_token,
            username,
        } => handle_join_game(
            config,
            gameplay_peer,
            registry,
            connection,
            protocol_version,
            session_token,
            username,
        ),

        ClientGameMessage::LeaveGame => handle_leave_game(gameplay_peer, registry, connection),

        ClientGameMessage::Heartbeat => ServerGameMessage::HeartbeatAck,

        ClientGameMessage::PlayerInput {
            movement_x,
            movement_y,
        } => handle_player_input(gameplay_peer, registry, connection, movement_x, movement_y),
    }
}