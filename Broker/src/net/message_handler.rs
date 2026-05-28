use crate::net::peer_roles::{
    PeerRole, PeerRoles
};
use crate::net::relay::*;
use bytes::Bytes;
use crate::pubsub::state::PubSubState;
use shared::game_sockets::{
    GameConnection, GamePeer, GameStream
};
use shared::protocol::broker::{BrokerMessage, decode_message, topic_to_string, encode_message, topic_for_shard};
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

        BrokerMessage::ClientHello { username: _ } => {
            if !peer_roles.register(connection, PeerRole::Client, "ClientHello") {
                return;
            }


            let client_id = state.allocate_client_id();
            state.register_client_connection(client_id, connection);

            let packet = match encode_message(&BrokerMessage::ClientAccepted {
                client_id,
            }) {
                Ok(packet) => packet,
                Err(error) => {
                    tracing::warn!(
                            "failed to encode ClientAccepted for connection {}: {}",
                            connection.connection_id,
                            error
                        );
                    return;
                }
            };

            if let Err(error) = peer.send(
                &connection,
                &stream,
                Bytes::from(packet)
            ) {
                tracing::warn!(
                        "failed to send ClientAccepted to connection {}: {}",
                        connection.connection_id,
                        error
                    );
                return;
            }

            tracing::info!(
                    "accepted client connection={} client_id={}",
                    connection.connection_id,
                    client_id
                );
        }

        BrokerMessage::RegisterClient { client_id } => {
            tracing::warn!(
                "legacy RegisterClient received from connection {}; assigning requested client_id={}",
                connection.connection_id,
                client_id
            );

            if !peer_roles.register(connection, PeerRole::Client, "RegisterClient") {
                return;
            }

            state.register_client_connection(client_id, connection);
        }

        BrokerMessage::RegisterShard { topic } => {
            if !peer_roles.register(connection, PeerRole::Shard, "RegisterShard") {
                return;
            }
        }

        BrokerMessage::RegisterSpatialService => {
            if !peer_roles.register(
                connection,
                PeerRole::SpatialService,
                "RegisterSpatialService",
            ) {
                return;
            }

            state.register_spatial_service(connection, stream);


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
            state.set_client_authority(client_id, topic);
            relay_add_client_to_shard(peer, state, topic, client_id, &payload);
        }

        BrokerMessage::SetClientAuthority { client_id, topic } => {
            if !peer_roles.ensure(connection, PeerRole::SpatialService, "SetClientAuthority") {
                return;
            }

            if !state.shard_streams_by_topic.contains_key(&topic) {
                tracing::warn!(
                    "rejected SetClientAuthority for client {}: unknown shard topic {}",
                    client_id,
                    topic_to_string(&topic)
                );
                return;
            }

            state.subscribe_registered_client(client_id, topic);
            state.set_client_authority(client_id, topic);
        }

        BrokerMessage::ClientAccepted { client_id } => {
            tracing::warn!(
                "broker received unexpected ClientAccepted from connection {} for client_id={}",
                connection.connection_id,
                client_id
            );
        },
        BrokerMessage::PositionUpdate { client_id , position} => {

            if !peer_roles.ensure(connection, PeerRole::Shard, "PositionUpdate") {
                return;
            }

            relay_position_update_to_spatial_services(
                peer,
                state,
                client_id,
                position,
            );
        }
    }
}