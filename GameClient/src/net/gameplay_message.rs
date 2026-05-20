use crate::net::gameplay_quic::GameplayClient;
use crate::net::input::handle_input_accepted;
use crate::net::login::{handle_goodbye, handle_join_accepted, handle_join_rejected};
use crate::world::state::LocalWorldState;
use shared::protocol::ServerGameMessage;

pub fn handle_server_message(
    gameplay_client: &mut GameplayClient,
    world_state: &mut LocalWorldState,
    message: ServerGameMessage,
) {
    match message {
        ServerGameMessage::JoinAccepted {
            player_id,
            player,
            snapshot,
            message,
        } => {
            handle_join_accepted(
                gameplay_client,
                world_state,
                player_id,
                player,
                snapshot,
                message,
            );
        }

        ServerGameMessage::JoinRejected { reason } => {
            handle_join_rejected(reason);
        }

        ServerGameMessage::HeartbeatAck => {
            tracing::info!("heartbeat ack");
        }

        ServerGameMessage::InputAccepted {
            movement_x,
            movement_y,
        } => {
            handle_input_accepted(world_state, movement_x, movement_y);
        }

        ServerGameMessage::WorldSnapshot { snapshot } => {
            world_state.zone = Some(snapshot.zone.clone());
            world_state.players = snapshot
                .players
                .iter()
                .map(|player| (player.player_id.clone(), player.clone()))
                .collect();

            tracing::info!(
                "world snapshot: zone={} players={}",
                snapshot.zone,
                snapshot.players.len()
            );
        }

        ServerGameMessage::PlayerJoined { player } => {
            tracing::info!(
                "player joined: id={} username={}",
                player.player_id,
                player.username
            );
        }

        ServerGameMessage::PlayerLeft { player_id } => {
            world_state.players.remove(&player_id);

            tracing::info!("player left: {}", player_id);
        }

        ServerGameMessage::Goodbye => {
            handle_goodbye(gameplay_client);
        }
    }
}