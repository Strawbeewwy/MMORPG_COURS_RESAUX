use crate::net::broker_client::BrokerClient;
use crate::world::state::LocalWorldState;
use shared::protocol::broker::{
    BrokerMessage, decode_message
};
use shared::protocol::transport::codec;
use shared::protocol::WorldUpdate;

pub fn decode_and_handle_broker_message(
    broker_client: &mut BrokerClient,
    world_state: &mut LocalWorldState,
    data: &[u8],
) {
    let broker_message = match decode_message(data) {
        Ok(message) => message,
        Err(error) => {
            tracing::warn!("failed to decode BrokerMessage: {error:#}");
            return;
        }
    };

    match broker_message {
        BrokerMessage::ClientAccepted { client_id } => {
            broker_client.client_id = Some(client_id);
            world_state.player_id = Some(client_id.into());

            tracing::info!("broker assigned client_id={}", client_id.0);
        }

        BrokerMessage::Broadcast { payload } => {
            decode_and_handle_world_update(broker_client, world_state, &payload);
        }

        other => {
            tracing::warn!("unexpected broker message received by client: {:?}", other);
        }
    }
}

fn decode_and_handle_world_update(
    _broker_client: &mut BrokerClient,
    world_state: &mut LocalWorldState,
    payload: &[u8],
) {
    let update = match codec::decode::<WorldUpdate>(payload) {
        Ok(update) => update,
        Err(error) => {
            tracing::warn!("failed to decode broadcast payload as WorldUpdate: {error:#}");
            return;
        }
    };

    handle_world_update(world_state, update);
}

fn handle_world_update(
    world_state: &mut LocalWorldState,
    update: WorldUpdate,
) {
    match update {
        WorldUpdate::Snapshot { snapshot } => {
            world_state.zone = Some(snapshot.zone.clone());
            world_state.set_players_from_snapshot(snapshot.players.clone());
            world_state.rebuild_render_entities();

            tracing::info!(
                "world snapshot: zone={} players={}",
                snapshot.zone,
                snapshot.players.len()
            );
        }

        WorldUpdate::PlayerJoined { player } => {
            tracing::info!(
                "player joined: id={} username={}",
                player.player_id,
                player.username
            );
        }

        WorldUpdate::PlayerLeft { player_id } => {
            world_state.players.remove(&player_id);
            world_state.rebuild_render_entities();

            tracing::info!("player left: {}", player_id);
        }
    }
}