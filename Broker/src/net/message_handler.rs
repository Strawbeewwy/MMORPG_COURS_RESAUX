use crate::net::peer_roles::{
    PeerRole, PeerRoles
};
use crate::net::relay::*;
use bytes::Bytes;
use crate::pubsub::state::{PubSubState};
use shared::game_sockets::{
    GameConnection, GamePeer, GameStream
};
use shared::protocol::*;

pub fn handle_message(
    peer: &GamePeer,
    peer_roles: &mut PeerRoles,
    state: &mut PubSubState,
    connection: GameConnection,
    stream: GameStream,
    data: &[u8],
) {
    let mut input = data;

    let tag = match input.first(){
        Some(tag) => *tag,
        None => return,
    };

    match tag {
        TAG_SUBSCRIBE => handle_subscribe_client(peer,peer_roles,state,&connection,&mut input),
        TAG_UNSUBSCRIBE => handle_unsubscribe_client(peer,peer_roles,state,&connection,&stream,&mut input),
        TAG_PUBLISH => relay_to_client(peer, state, &connection, &stream, data),
        TAG_CLIENT_INPUT => relay_client_input_to_shard(peer,state,&connection,&stream,data),
        TAG_REGISTER_SHARD => handle_register_shard(peer,peer_roles,state,&connection,&stream,&mut input),
        TAG_REGISTER_SPATIAL_SERVICE => handle_register_spatial_service(peer,peer_roles,state,&connection,&stream),
        TAG_CLIENT_HELLO => handle_client_hello(peer,peer_roles,state,&connection,&stream,&mut input),
        TAG_REQUEST_ENTITY_ID_BLOCK => handle_request_entity_id_block(peer, peer_roles, state, &connection, &stream, data),
        TAG_ENTITY_ID_BLOCK_ALLOCATED => relay_entity_id_block_allocated_to_shard(peer, state, &connection, &stream, data.clone()),
        TAG_POSITION_UPDATE => relay_to_spatial_services(peer,state,data),
        TAG_HANDOFF_REQUEST => relay_handoff_request_to_shards(peer, state, &connection, &stream, data),
        TAG_HANDOFF_ACCEPTED => relay_handoff_accepted_to_shard(peer, state, &connection, &stream, data),
        TAG_HANDOFF_REJECTED => relay_handoff_rejected_to_shard(peer, state, &connection, &stream, data),
        TAG_HANDOFF_COMPLETE => relay_handoff_completed_to_shard(peer, state, &connection, &stream, data),
        TAG_GHOST_UPDATE => relay_ghost_update(peer, state, &connection, &stream, data),
        TAG_HANDOFF_START => relay_handoff_start_to_shards(peer,peer_roles, state, &connection, &stream, data),
        TAG_REGISTER_ENTITY => relay_to_spatial_services(peer, state, data),
        TAG_UNREGISTER_ENTITY => relay_to_spatial_services(peer, state, data),
        _ => {
            tracing::warn!(
                "unknown tag or unexpected tag for message sent by : {}",
                connection.connection_id
            );
        }
    };
}

fn handle_register_shard(
    peer: &GamePeer,
    peer_roles: &mut PeerRoles,
    state: &mut PubSubState,
    connection: &GameConnection,
    stream: &GameStream,
    input: &mut &[u8],
){
    if !peer_roles.register_role(
        *connection,
        PeerRole::Shard,
        "RegisterShard"
    ) {
        return;
    }

    let message = match decode_message(input) {
        Ok(message) => message,
        Err(error) => {
            tracing::warn!(
                "could not decode message {}: {error}",
                connection.connection_id
            );
            return;
        }
    };

    match message {

        NetworkMessage::RegisterShard { shard_id } => {
            state.register_shard_topic(shard_id, *connection, stream.clone());

            tracing::info!(
                "registered shard connection={} shard_id={}",
                connection.connection_id,
                shard_id.0,
            );

            let packet = match encode_message(&NetworkMessage::RegisterShard { shard_id }) {
                Ok(packet) => packet,
                Err(error) => {
                    tracing::warn!(
                        "failed to encode RegisterShard notification for shard {}: {}",
                        shard_id.0,
                        error
                    );
                    return;
                }
            };

            relay_to_spatial_services(peer, state, &packet);
        }
        _ => {
            tracing::warn!(
                "invalid message Register Shard sent by : {}",
                connection.connection_id
            );
        }
    }
}

fn handle_request_entity_id_block(
    peer: &GamePeer,
    peer_roles: &mut PeerRoles,
    state: &mut PubSubState,
    connection: &GameConnection,
    stream: &GameStream,
    data: &[u8],
) {
    if !peer_roles.ensure(*connection, PeerRole::Shard, "RequestEntityIdBlock") {
        tracing::warn!(
            "RequestEntityIdBlock not from a shard: {}",
            connection.connection_id
        );
        return;
    }

    let message = match decode_message(data) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("could not decode RequestEntityIdBlock: {e}");
            return;
        }
    };

    if let NetworkMessage::RequestEntityIdBlock { shard_id, .. } = message {
        state.push_entity_id_block_request(shard_id);
        relay_to_spatial_services(peer, state, data);
    }
}

fn handle_register_spatial_service(
    peer: &GamePeer,
    peer_roles: &mut PeerRoles,
    state: &mut PubSubState,
    connection: &GameConnection,
    stream: &GameStream,
){
    if !peer_roles.register_role(
        *connection,
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


fn handle_subscribe_client(
    peer: &GamePeer,
    peer_roles: &mut PeerRoles,
    state: &mut PubSubState,
    connection: &GameConnection,
    input: &mut &[u8],
) {

    if !peer_roles.ensure(*connection, PeerRole::SpatialService, "Subscribe") {
        tracing::warn!(
                "message not received from Spatial service {}:",
                connection.connection_id
            );
        return;
    }

    let message = match decode_message(input) {
        Ok(message) => message,
        Err(error) => {
            tracing::warn!(
                "could not decode message {}: {error}",
                connection.connection_id
            );
            return;
        }
    };

    match message {
        NetworkMessage::Subscribe { client_id, topic, } => {

            state.subscribe_registered_client(client_id, topic);

            let shard_connection = match state.shard_streams_by_topic.get_by_left(&topic){
                Some(connection) => connection,
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
                &shard_connection.connection,
                &shard_connection.stream,
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
        _ => {
            tracing::warn!(
                "invalid message Subscribe Client sent by : {}",
                connection.connection_id
            );
        }
    }
}

fn handle_unsubscribe_client(
    peer: &GamePeer,
    peer_roles: &mut PeerRoles,
    state: &mut PubSubState,
    connection: &GameConnection,
    stream: &GameStream,
    input: &mut &[u8],
) {


    if !peer_roles.ensure(*connection, PeerRole::SpatialService, "Subscribe") {
        tracing::warn!(
                "message not received from Spatial service {}:",
                connection.connection_id
            );
        return;
    }

    let message = match decode_message(input) {
        Ok(message) => message,
        Err(error) => {
            tracing::warn!(
                "could not decode message {}: {error}",
                connection.connection_id
            );
            return;
        }
    };

    match message {
        NetworkMessage::Unsubscribe { client_id, topic } => {
            state.unsubscribe_client(client_id, topic);
        }
        _ => {
            tracing::warn!(
                "invalid message Unsubscribe Client sent by : {}",
                connection.connection_id
            );
        }
    }

    //TODO Tell shard to unregister client


}


fn handle_client_hello(
    peer: &GamePeer,
    peer_roles: &mut PeerRoles,
    state: &mut PubSubState,
    connection: &GameConnection,
    stream: &GameStream,
    input: &mut &[u8],
) {
    if !peer_roles.register_role(*connection, PeerRole::Client, "ClientHello") {
        tracing::warn!(
                "message not received from client {}:",
                connection.connection_id
            );
        return;
    }

    let message = match decode_message(input) {
        Ok(message) => message,
        Err(error) => {
            tracing::warn!(
                "could not decode message {}: {error}",
                connection.connection_id
            );
            return;
        }
    };


    match message {
        NetworkMessage::ClientHello { username } => {


            let client_id = state.allocate_client_id();
            state.register_client_connection(&client_id, &username, &connection, &stream);

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
                connection,
                stream,
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
        _ => {
            tracing::warn!(
                "invalid tag for message Client Hello sent by : {}",
                connection.connection_id
            );
        }
    }
}






