use crate::net::peer_roles::{
    PeerRole, PeerRoles
};
use crate::net::relay::*;
use bytes::Bytes;
use crate::pubsub::state::PubSubState;
use shared::game_sockets::{
    GameConnection, GamePeer, GameStream
};
use shared::protocol::{NetworkMessage, decode_message, encode_message, Topic};
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
                "invalid utils message from connection {}: {error}",
                connection.connection_id
            );
            return;
        }
    };

    match message {

        NetworkMessage::ClientHello { username: _ } => {
            if !peer_roles.register_role(connection, PeerRole::Client, "ClientHello") {
                return;
            }


            let client_id = state.allocate_client_id();
            state.register_client_connection(client_id, connection);

            let packet = match encode_message(&NetworkMessage::ClientAccepted {
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
                    client_id.0
                );
        }

        NetworkMessage::RegisterShard { shard_id } => {
            if !peer_roles.register_role(
                connection,
                PeerRole::Shard,
                "RegisterShard"
            ) {
                return;
            }

            state.register_shard_topic(shard_id, connection, stream);

            tracing::info!(
                "registered shard connection={}",
                connection.connection_id
            );
        }

        NetworkMessage::RegisterSpatialService => {
            if !peer_roles.register_role(
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

        NetworkMessage::Subscribe { client_id, shard_id } => {
            if !peer_roles.ensure(connection, PeerRole::SpatialService, "Subscribe") {
                return;
            }

            state.subscribe_registered_client(client_id, shard_id);
        }

        NetworkMessage::Unsubscribe { client_id, shard_id } => {
            if !peer_roles.ensure(connection, PeerRole::SpatialService, "Unsubscribe") {
                return;
            }

            state.unsubscribe_client(client_id, shard_id);
        }

        NetworkMessage::Publish { shard_id, client_id, payload_len,payload } => {
            if !peer_roles.ensure(
                connection,
                PeerRole::Shard,
                "Publish"
            ) {
                return;
            }

            if payload.len() != payload_len.clone() as usize {
                tracing::warn!("received payload does not match it's expected length");
                return;
            }


            let topic = Topic::ShardInstance(shard_id);

            publish_to_client(
                peer,
                reliable_streams,
                state,
                topic,
                client_id,
                payload_len,
                &payload,
            );
        }

        NetworkMessage::ClientInput { client_id, input } => {
            if !peer_roles.ensure(
                connection,
                PeerRole::Client,
                "ClientInput") {
                return;
            }

            relay_client_input_to_shard(
                peer,
                state,
                client_id,
                input);
        }

        NetworkMessage::Broadcast { .. } => {
            tracing::warn!(
                "utils received unexpected Broadcast message from connection {}",
                connection.connection_id
            );
        }

        NetworkMessage::ClientAccepted { client_id } => {
            tracing::warn!(
                "utils received unexpected ClientAccepted from connection {} for client_id={}",
                connection.connection_id,
                client_id.0
            );
        },
        NetworkMessage::PositionUpdate { client_id , position} => {

            if !peer_roles.ensure(
                connection,
                PeerRole::Shard,
                "PositionUpdate") {
                return;
            }

            relay_position_update_to_spatial_services(
                peer,
                state,
                client_id,
                position,
            );
        }
        //from broker to shard,  
        // we need to register the client to the shard
        // for AOI and client inputs
        NetworkMessage::RegisterClient { .. } => {}
        //from spatial to broker then to shards
        NetworkMessage::HandoffRequest { .. } => {}
        //from shard to broker then to spatial
        NetworkMessage::HandoffAccepted { .. } => {}
        //from shard to broker then to spatial
        NetworkMessage::HandoffRejected { .. } => {}
        //from a shard to another shard
        NetworkMessage::GhostUpdate { .. } => {}
        //from shard to spatial
        NetworkMessage::HandoffCompleted { .. } => {}
    }
}