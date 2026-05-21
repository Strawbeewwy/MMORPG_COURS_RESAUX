use crate::config::BrokerConfig;
use crate::pubsub::state::PubSubState;
use bevy::prelude::*;
use bytes::Bytes;
use shared::game_sockets::protocols::QuicBackend;
use shared::game_sockets::{
    GameConnection, GameNetworkEvent, GamePeer, GameStream, GameStreamReliability,
};
use shared::protocol::broker::{
    BrokerMessage, CLIENT_INPUT_LEN, Topic, decode_message, encode_broadcast,
    encode_client_input, topic_to_string,
};
use std::collections::HashMap;

#[derive(Resource)]
pub struct BrokerPeer {
    pub peer: GamePeer,
    pub reliable_streams: HashMap<GameConnection, GameStream>,
}

pub fn start_broker(mut commands: Commands, config: Res<BrokerConfig>) {
    let peer = GamePeer::new(QuicBackend::new());

    peer.listen("0.0.0.0", config.port)
        .expect("failed to start broker QUIC listener");

    tracing::info!("broker listening on 0.0.0.0:{}", config.port);

    commands.insert_resource(BrokerPeer {
        peer,
        reliable_streams: HashMap::new(),
    });
}

pub fn poll_broker_events(
    mut broker: ResMut<BrokerPeer>,
    mut state: ResMut<PubSubState>,
) {
    loop {
        let event = match broker.peer.poll() {
            Ok(Some(event)) => event,
            Ok(None) => break,
            Err(error) => {
                tracing::error!("failed to poll broker peer: {error}");
                break;
            }
        };

        handle_broker_event(&mut broker, &mut state, event);
    }
}

fn handle_broker_event(
    broker: &mut BrokerPeer,
    state: &mut PubSubState,
    event: GameNetworkEvent,
) {
    match event {
        GameNetworkEvent::Connected(connection) => {
            tracing::info!("peer connected to broker: {}", connection.connection_id);

            if let Err(error) = broker
                .peer
                .create_stream(connection, GameStreamReliability::Reliable)
            {
                tracing::error!(
                    "failed to create reliable stream for connection {}: {}",
                    connection.connection_id,
                    error
                );
            }
        }

        GameNetworkEvent::Disconnected(connection) => {
            tracing::info!("peer disconnected from broker: {}", connection.connection_id);

            broker.reliable_streams.remove(&connection);
            state.remove_connection(connection);
        }

        GameNetworkEvent::StreamCreated(connection, stream) => {
            tracing::info!(
                "broker stream created: connection={} stream={}",
                connection.connection_id,
                stream.stream_id
            );

            if stream.is_reliable() {
                broker.reliable_streams.insert(connection, stream);
            }
        }

        GameNetworkEvent::StreamClosed(connection, stream) => {
            tracing::info!(
                "broker stream closed: connection={} stream={}",
                connection.connection_id,
                stream.stream_id
            );

            broker.reliable_streams.remove(&connection);
        }

        GameNetworkEvent::Message {
            connection,
            stream,
            data,
        } => {
            handle_broker_message(broker, state, connection, stream, &data);
        }

        GameNetworkEvent::Error { connection, inner } => {
            tracing::warn!(
                "broker socket error on connection {}: {}",
                connection.connection_id,
                inner
            );
        }
    }
}

fn handle_broker_message(
    broker: &mut BrokerPeer,
    state: &mut PubSubState,
    connection: GameConnection,
    stream: GameStream,
    data: &[u8],
) {
    let message = match decode_message(data) {
        Ok(message) => message,
        Err(error) => {
            tracing::warn!(
                "invalid broker message from connection {}: {error}",
                connection.connection_id
            );
            return;
        }
    };

    match message {
        BrokerMessage::Subscribe { client_id, topic } => {
            state.subscribe_client(client_id, topic, connection);
        }

        BrokerMessage::Unsubscribe { client_id, topic } => {
            state.unsubscribe_client(client_id, topic);
        }

        BrokerMessage::Publish { topic, payload } => {
            state.register_shard_topic(topic, connection, stream);
            publish_to_subscribers(broker, state, topic, &payload);
        }

        BrokerMessage::ClientInput { client_id, input } => {
            state.register_client_connection(client_id, connection);
            relay_client_input_to_shard(broker, state, client_id, input);
        }
        _ => {}
    }
}

fn publish_to_subscribers(
    broker: &BrokerPeer,
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

        let Some(stream) = broker.reliable_streams.get(connection) else {
            continue;
        };

        if let Err(error) = broker.peer.send(connection, stream, Bytes::from(packet.clone())) {
            tracing::warn!(
                "failed to send broadcast to client {} on connection {}: {}",
                client_id,
                connection.connection_id,
                error
            );
        }
    }
}

fn relay_client_input_to_shard(
    broker: &BrokerPeer,
    state: &PubSubState,
    client_id: u32,
    input: [u8; CLIENT_INPUT_LEN],
) {
    let Some(topic) = state.first_topic_for_client(client_id) else {
        tracing::warn!("cannot relay input: client {} has no topic", client_id);
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

    if let Err(error) = broker
        .peer
        .send(shard_connection, shard_stream, Bytes::from(packet))
    {
        tracing::warn!(
            "failed to relay input from client {} to shard topic {}: {}",
            client_id,
            topic_to_string(&topic),
            error
        );
    }
}