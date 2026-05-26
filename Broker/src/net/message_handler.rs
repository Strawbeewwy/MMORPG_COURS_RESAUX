use crate::net::peer_roles::{PeerRole, PeerRoles};
use crate::net::relay::{
    publish_to_subscribers, relay_add_client_to_shard, relay_client_input_to_shard,
};
use crate::pubsub::state::PubSubState;
use shared::game_sockets::{GameConnection, GamePeer, GameStream};
use shared::protocol::broker::{BrokerMessage, decode_message, topic_to_string};
use std::collections::HashMap;

pub fn handle_message(
    peer: &GamePeer,
    reliable_streams: &HashMap<GameConnection, GameStream>,
    peer_roles: &mut PeerRoles,
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
        BrokerMessage::RegisterClient { client_id } => {
            if !peer_roles.register(connection, PeerRole::Client, "RegisterClient") {
                return;
            }

            state.register_client_connection(client_id, connection);
        }

        BrokerMessage::RegisterShard { topic } => {
            if !peer_roles.register(connection, PeerRole::Shard, "RegisterShard") {
                return;
            }

            state.register_shard_topic(topic, connection, stream);

            tracing::info!(
                "registered shard connection={} topic={}",
                connection.connection_id,
                topic_to_string(&topic)
            );
        }

        BrokerMessage::RegisterSpatialService => {
            if !peer_roles.register(
                connection,
                PeerRole::SpatialService,
                "RegisterSpatialService",
            ) {
                return;
            }

            tracing::info!(
                "registered spatial service connection={}",
                connection.connection_id
            );
        }

        BrokerMessage::Subscribe { client_id, topic } => {
            if !peer_roles.ensure(connection, PeerRole::SpatialService, "Subscribe") {
                return;
            }

            state.subscribe_registered_client(client_id, topic);
        }

        BrokerMessage::Unsubscribe { client_id, topic } => {
            if !peer_roles.ensure(connection, PeerRole::SpatialService, "Unsubscribe") {
                return;
            }

            state.unsubscribe_client(client_id, topic);
        }

        BrokerMessage::Publish { topic, payload } => {
            if !peer_roles.ensure(connection, PeerRole::Shard, "Publish") {
                return;
            }

            state.register_shard_topic(topic, connection, stream);
            publish_to_subscribers(peer, reliable_streams, state, topic, &payload);
        }

        BrokerMessage::ClientInput { client_id, input } => {
            if !peer_roles.ensure(connection, PeerRole::Client, "ClientInput") {
                return;
            }

            state.register_client_connection(client_id, connection);
            relay_client_input_to_shard(peer, state, client_id, input);
        }

        BrokerMessage::Broadcast { .. } => {
            tracing::warn!(
                "broker received unexpected Broadcast message from connection {}",
                connection.connection_id
            );
        }

        BrokerMessage::AddClientToShard {
            topic,
            client_id,
            payload,
        } => {
            if !peer_roles.ensure(connection, PeerRole::SpatialService, "AddClientToShard") {
                return;
            }

            state.subscribe_registered_client(client_id, topic);
            relay_add_client_to_shard(peer, state, topic, client_id, &payload);
        }
    }
}