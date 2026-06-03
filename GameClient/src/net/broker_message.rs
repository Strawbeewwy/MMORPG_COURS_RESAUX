use crate::net::broker_client::BrokerClient;
use crate::world::state::LocalWorldState;
use shared::protocol::{
    NetworkMessage, decode_message
};
use shared::protocol::http::codec;
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
        NetworkMessage::ClientAccepted { client_id } => {
            broker_client.client_id = Some(client_id);

            tracing::info!("utils assigned client_id={}", client_id.0);
        }

        NetworkMessage::Broadcast { payload_len, payload } => {
            decode_and_handle_world_update(broker_client, world_state,&payload_len, &payload);
        }

        other => {
            tracing::warn!("unexpected utils message received by client: {:?}", other);
        }
    }
}

fn decode_and_handle_world_update(
    _broker_client: &mut BrokerClient,
    world_state: &mut LocalWorldState,
    payload_len: &u16,
    payload: &[u8],
) {

    if payload.len() != payload_len.clone() as usize {
        tracing::warn!("received payload does not match it's expected length");
        return;
    }
    

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

        WorldUpdate::PlayerJoined { player, client_id } => {
            tracing::info!(
                "player joined: id={} username={}",
                client_id.0.clone(),
                player.username
            );
        }

        WorldUpdate::PlayerLeft { player, client_id } => {
            //world_state.players.remove();
            world_state.rebuild_render_entities();

            tracing::info!("player left: id = {} username={}",
                client_id.0.clone(),
                player.username
            );
        }
    }
}