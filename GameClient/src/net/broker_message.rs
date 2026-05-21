use crate::net::broker_client::BrokerClient;
use crate::world::state::LocalWorldState;
use crate::net::gameplay_message::handle_server_message;
use shared::protocol::broker::{BrokerMessage, decode_message};
use shared::protocol::transport::codec;
use shared::protocol::ServerGameMessage;

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
        BrokerMessage::Broadcast { payload } => {
            decode_and_handle_broadcast_payload(broker_client, world_state, &payload);
        }

        other => {
            tracing::warn!("unexpected broker message received by client: {:?}", other);
        }
    }
}

fn decode_and_handle_broadcast_payload(
    broker_client: &mut BrokerClient,
    world_state: &mut LocalWorldState,
    payload: &[u8],
) {
    let message = match codec::decode::<ServerGameMessage>(payload) {
        Ok(message) => message,
        Err(error) => {
            tracing::warn!("failed to decode broadcast payload as ServerGameMessage: {error:#}");
            return;
        }
    };

    handle_server_message(broker_client, world_state, message);
}