use crate::net::peer_roles::{
    PeerRole, PeerRoles
};
use crate::net::relay::*;
use bytes::Bytes;
use crate::pubsub::state::PubSubState;
use shared::game_sockets::{
    GameConnection, GamePeer, GameStream
};
use shared::protocol::{NetworkMessage, decode_message, encode_message, Topic, EntityId};

pub fn handle_message(
    peer: &GamePeer,
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
                "invalid message from connection {}: {error}",
                connection.connection_id
            );
            return;
        }
    };

    match message {

        NetworkMessage::ClientHello { username } => {
            if !peer_roles.register_role(connection, PeerRole::Client, "ClientHello") {
                return;
            }


            let client_id = state.allocate_client_id();
            state.register_client_connection(&client_id,&username, &connection ,&stream);

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

        NetworkMessage::Subscribe { client_id, topic } => {
            if !peer_roles.ensure(connection, PeerRole::SpatialService, "Subscribe") {
                return;
            }

            state.subscribe_registered_client(client_id, topic);

            let (shard_connection,shard_stream) = match state.shard_streams_by_topic.get(&topic){
                Some((connection,stream)) => (connection,stream),
                None => {
                    tracing::warn!("no shard connection found for topic: {:?}", topic);
                    return;
                }
            };

            let username = match state.client_username.get(&client_id){
                Some(username) => username,
                None => {
                    tracing::warn!("no username found for client_id: {}", client_id.0);
                    return;
                }
            };

            let packet  = match encode_message(&NetworkMessage::RegisterClient { client_id, username: username.clone() }){
                Ok(packet) => packet,
                Err(error) => {
                    tracing::warn!(
                            "failed to encode subscribe message for connection {}: {}",
                            connection.connection_id,
                            error
                        );
                    return;
                }
            };

            if let Err(error) = peer.send(
                &shard_connection,
                &shard_stream,
                Bytes::from(packet)
            ) {
                tracing::warn!(
                        "failed to send register client to connection {}: {}",
                        connection.connection_id,
                        error
                    );
                return;
            }

        }

        NetworkMessage::Unsubscribe { client_id, topic } => {
            if !peer_roles.ensure(connection, PeerRole::SpatialService, "Unsubscribe") {
                return;
            };

            state.unsubscribe_client(client_id, topic);
        }

        NetworkMessage::Publish { topic, payload_len,payload } => {
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

            relay_to_client(
                peer,
                state,
                topic,
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

        NetworkMessage::ClientAccepted { client_id } => {
            tracing::warn!(
                "utils received unexpected ClientAccepted from connection {} for client_id={}",
                connection.connection_id,
                client_id.0
            );
        },
        NetworkMessage::PositionUpdate { entity_id, position } => {
            if !peer_roles.ensure(
                connection,
                PeerRole::Shard,
                "PositionUpdate"
            ) {
                return;
            }

            relay_position_update_to_spatial_services(
                peer,
                state,
                entity_id,
                position,
            );
        }
        //from spatial to broker then to shards
        NetworkMessage::HandoffRequest { entity_id,from_shard_id,to_shard_id,position,velocity,entity_state } => {

            relay_handoff_request_to_shards(
                peer,
                state,
                entity_id,
                from_shard_id,
                to_shard_id,
                position,
                velocity,
                entity_state,
            );
        }
        //from shard to broker then to spatial
        NetworkMessage::HandoffAccepted{entity_id} => {

            if !peer_roles.ensure(
                connection,
                PeerRole::Shard,
                "HandoffAccepted") {
                return;
            }


            relay_handoff_accepted_to_spatial(
                peer,
                state,
                entity_id,
            );
        }
        //from shard to broker then to spatial
        NetworkMessage::HandoffRejected { entity_id } => {

            if !peer_roles.ensure(
                connection,
                PeerRole::Shard,
                "HandoffRejected") {
                return;
            }

            relay_handoff_rejected_to_spatial(
                peer,
                state,
                entity_id,
            );
        }
        //from a shard to another shard
        NetworkMessage::GhostUpdate { entity_id,position,velocity } => {}
        //from spatial to shard
        NetworkMessage::HandoffCompleted { entity_id } => {
            if !peer_roles.ensure(
                connection,
                PeerRole::SpatialService,
                "HandoffCompleted") {
                return;
            }

            relay_handoff_completed_to_shard(
                peer,
                state,
                entity_id,
            );

        }
        NetworkMessage::RequestEntityIdBlock { shard_id, count } => {
            if !peer_roles.ensure(
                connection,
                PeerRole::Shard,
                "RequestEntityIdBlock"
            ) {
                return;
            }

            relay_entity_id_block_request_to_spatial(
                peer,
                state,
                shard_id,
                count,
            );
        }

        NetworkMessage::EntityIdBlockAllocated {
            shard_id,
            start,
            count,
        } => {
            if !peer_roles.ensure(
                connection,
                PeerRole::SpatialService,
                "EntityIdBlockAllocated"
            ) {
                return;
            }

            relay_entity_id_block_allocated_to_shard(
                peer,
                state,
                shard_id,
                start,
                count,
            );
        }

        _ => {
            tracing::warn!(
                "broker received unexpected message from connection {}",
                connection.connection_id
            );
        }
    }
}




