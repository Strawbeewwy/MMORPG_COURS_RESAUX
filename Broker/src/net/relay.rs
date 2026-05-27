use crate::pubsub::state::PubSubState;
use bytes::Bytes;
use shared::game_sockets::{GameConnection, GamePeer, GameStream};
use shared::protocol::broker::{
    CLIENT_INPUT_LEN, ClientId, Topic, encode_add_client_to_shard, encode_broadcast,
    encode_client_input, topic_to_string,
};
use std::collections::HashMap;


pub fn publish_to_subscribers(
    peer: &GamePeer,
    reliable_streams: &HashMap<GameConnection, GameStream>,
    state: &PubSubState,
    topic: Topic,
    payload: &[u8],
) {
    let Some(subscribers) = state.topic_subscribers.get(&topic) else {
        return;
    };

    let packet = match encode_broadcast(payload) {
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
                client_id,
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
            client_id
        );
        return;
    };

    let Some((shard_connection, shard_stream)) = state.shard_streams_by_topic.get(&topic) else {
        tracing::warn!(
            "cannot relay input: no shard known for topic {}",
            topic_to_string(&topic)
        );
        return;
    };

    let packet = encode_client_input(client_id, input);

    if let Err(error) = peer.send(shard_connection, shard_stream, Bytes::from(packet)) {
        tracing::warn!(
            "failed to relay input from client {} to shard topic {}: {}",
            client_id,
            topic_to_string(&topic),
            error
        );
    }
}

pub fn relay_add_client_to_shard(
    peer: &GamePeer,
    state: &PubSubState,
    topic: Topic,
    client_id: ClientId,
    payload: &[u8],
) {
    let Some((shard_connection, shard_stream)) = state.shard_streams_by_topic.get(&topic) else {
        tracing::warn!(
            "cannot add client {} to shard: no shard known for topic {}",
            client_id,
            topic_to_string(&topic)
        );
        return;
    };

    let packet = match encode_add_client_to_shard(topic, client_id, payload) {
        Ok(packet) => packet,
        Err(error) => {
            tracing::warn!(
                "cannot encode AddClientToShard for client {} topic {}: {}",
                client_id,
                topic_to_string(&topic),
                error
            );
            return;
        }
    };

    if let Err(error) = peer.send(shard_connection, shard_stream, Bytes::from(packet)) {
        tracing::warn!(
            "failed to relay AddClientToShard client {} to topic {}: {}",
            client_id,
            topic_to_string(&topic),
            error
        );
    }
}