use crate::pubsub::state::PubSubState;
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

pub struct BrokerNetwork {
    peer: GamePeer,
    reliable_streams: HashMap<GameConnection, GameStream>,
}

impl BrokerNetwork {
    pub fn listen(port: u16) -> anyhow::Result<Self> {
        let peer = GamePeer::new(QuicBackend::new());

        peer.listen("0.0.0.0", port)?;

        tracing::info!("broker listening on 0.0.0.0:{port}");

        Ok(Self {
            peer,
            reliable_streams: HashMap::new(),
        })
    }

    pub fn poll_events(&mut self, state: &mut PubSubState) {
        loop {
            let event = match self.peer.poll() {
                Ok(Some(event)) => event,
                Ok(None) => break,
                Err(error) => {
                    tracing::error!("failed to poll broker peer: {error}");
                    break;
                }
            };

            self.handle_event(state, event);
        }
    }

    fn handle_event(
        &mut self,
        state: &mut PubSubState,
        event: GameNetworkEvent,
    ) {
        match event {
            GameNetworkEvent::Connected(connection) => {
                tracing::info!("peer connected to broker: {}", connection.connection_id);

                if let Err(error) = self
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

                self.reliable_streams.remove(&connection);
                state.remove_connection(connection);
            }

            GameNetworkEvent::StreamCreated(connection, stream) => {
                tracing::info!(
                    "broker stream created: connection={} stream={}",
                    connection.connection_id,
                    stream.stream_id
                );

                if stream.is_reliable() {
                    self.reliable_streams.insert(connection, stream);
                }
            }

            GameNetworkEvent::StreamClosed(connection, stream) => {
                tracing::info!(
                    "broker stream closed: connection={} stream={}",
                    connection.connection_id,
                    stream.stream_id
                );

                self.reliable_streams.remove(&connection);
            }

            GameNetworkEvent::Message {
                connection,
                stream,
                data,
            } => {
                self.handle_message(state, connection, stream, &data);
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

    fn handle_message(
        &mut self,
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
                self.publish_to_subscribers(state, topic, &payload);
            }

            BrokerMessage::ClientInput { client_id, input } => {
                state.register_client_connection(client_id, connection);
                self.relay_client_input_to_shard(state, client_id, input);
            }

            BrokerMessage::Broadcast { .. } => {
                tracing::warn!(
                    "broker received unexpected Broadcast message from connection {}",
                    connection.connection_id
                );
            }
        }
    }

    fn publish_to_subscribers(
        &self,
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

            let Some(stream) = self.reliable_streams.get(connection) else {
                continue;
            };

            if let Err(error) = self.peer.send(connection, stream, Bytes::from(packet.clone())) {
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
        &self,
        state: &PubSubState,
        client_id: u32,
        input: [u8; CLIENT_INPUT_LEN],
    ) {
        let Some(topic) = state.first_shard_topic_for_client(client_id) else {
            tracing::warn!(
                "cannot relay input: client {} has no subscribed shard topic",
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

        if let Err(error) = self
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
}