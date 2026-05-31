use crate::pubsub::state::PubSubState;
use bytes::Bytes;
use shared::game_sockets::{GameConnection, GamePeer, GameStream};
use shared::protocol::broker::{
    CLIENT_INPUT_LEN, ClientId, Topic, encode_message,
     BrokerMessage, };
use std::collections::HashMap;


pub fn broadcast_to_subscribers(
    peer: &GamePeer,
    reliable_streams: &HashMap<GameConnection, GameStream>,
    state: &PubSubState,
    topic: Topic,
    payload_len: &u16,
    payload: &[u8],
) {
    let Some(subscribers) = state.topic_subscribers.get(&topic) else {
        return;
    };

    let packet = match encode_message(&BrokerMessage::Broadcast {
        payload_len: payload_len.clone(),
        payload: Vec::from(payload),
    }) {
        Ok(packet) => packet,
        Err(error) => {
            tracing::warn!("cannot encode broadcast: {error}");
            return;
        }
    };

    for client_id in subscribers {
        let Some(connection) = state.client_connections.get(client_id) else {
            continue;
        };

        let Some(stream) = reliable_streams.get(connection) else {
            continue;
        };

        if let Err(error) = peer.send(connection, stream, Bytes::from(packet.clone())) {
            tracing::warn!(
                "failed to send broadcast to client {} on connection {}: {}",
                client_id.0,
                connection.connection_id,
                error
            );
        }
    }
}

pub fn relay_client_input_to_shard(
    peer: &GamePeer,
    state: &PubSubState,
    client_id: ClientId,
    input: [u8; CLIENT_INPUT_LEN],
) {
    let Some(topic) = state.input_topic_for_client(client_id) else {
        tracing::warn!(
            "cannot relay input: client {} has no authoritative or subscribed shard topic",
            client_id.0
        );
        return;
    };

    let Some((shard_connection, shard_stream)) = state.shard_streams_by_topic.get(&topic) else {
        tracing::warn!(
            "cannot relay input: no shard known for topic {}",
            &topic.to_string()
        );
        return;
    };

    let packet = match encode_message(&BrokerMessage::ClientInput {
        client_id,
        input,
    }) {
        Ok(packet) => packet,
        Err(error) => {
            tracing::warn!("cannot encode broadcast: {error}");
            return;
        }
    };


    if let Err(error) = peer.send(shard_connection, shard_stream, Bytes::from(packet)) {
        tracing::warn!(
            "failed to relay input from client {} to shard topic {}: {}",
            client_id.0,
            &topic.to_string(),
            error
        );
    }
}


pub fn relay_position_update_to_spatial_services(
    peer : &GamePeer,
    state: &PubSubState,
    client_id: ClientId,
    position: [f32;2],
){

    if state.spatial_service_streams.is_empty() {
        tracing::warn!(
                    "cannot forward PositionUpdate for client {}: no spatial service registered",
                    client_id.0
                );
        return;
    }

    let packet = match encode_message(&BrokerMessage::PositionUpdate {
        client_id,
        position,
    }) {
        Ok(packet) => packet,
        Err(error) => {
            tracing::warn!(
                        "failed to encode PositionUpdate for client {}: {}",
                        client_id.0,
                        error
                    );
            return;
        }
    };

    for (spatial_connection, spatial_stream) in &state.spatial_service_streams {
        if let Err(error) = peer.send(
            spatial_connection,
            spatial_stream,
            Bytes::from(packet.clone()),
        ) {
            tracing::warn!(
                "failed to forward PositionUpdate for client {} to spatial service connection {}: {}",
                client_id.0,
                spatial_connection.connection_id,
                error
            );
        }
    }

    tracing::debug!(
        "forwarded PositionUpdate client_id={} position=({}, {}) to {} spatial service(s)",
        client_id.0,
        position[0],
        position[1],
        state.spatial_service_streams.len()
    );

}

