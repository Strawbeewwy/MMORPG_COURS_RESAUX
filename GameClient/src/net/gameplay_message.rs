use crate::net::broker_client::BrokerClient;
use crate::net::input::handle_input_accepted;
use crate::net::login::{handle_goodbye, handle_join_accepted, handle_join_rejected};
use crate::world::state::LocalWorldState;
use shared::protocol::ServerGameMessage;

pub fn handle_server_message(
    broker_client: &mut BrokerClient,
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
                broker_client,
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
            world_state.set_players_from_snapshot(snapshot.players.clone());
            world_state.rebuild_render_entities();

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
            world_state.rebuild_render_entities();

            tracing::info!("player left: {}", player_id);
        }

        ServerGameMessage::Goodbye => {
            handle_goodbye(broker_client);
        }
    }
}