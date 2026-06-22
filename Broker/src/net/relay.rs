use crate::pubsub::state::{ConnectionStream, GhostRoute, PubSubState};
use bytes::Bytes;
use game_sockets::{
    GameConnection, GameStream, GamePeer
};
use shared::{decode_message, encode_message, NetworkMessage, Topic};
use crate::net::peer_roles::{PeerRole, PeerRoles};

pub fn relay_to_client(
    peer: &GamePeer,
    state: &mut PubSubState,
    connection: &GameConnection,
    stream: &GameStream,
    data: &[u8],
) {
    // First, check if this is a Publish message with Topic::Client
    let message = match decode_message(data) {
        Ok(msg) => msg,
        Err(e) => {
            tracing::warn!("Failed to decode message in relay_to_client: {}", e);
            // Fallback to old behavior
            relay_shard_to_subscribers(peer, state, connection, stream, data);
            return;
        }
    };

    if let NetworkMessage::Publish { topic, payload, .. } = message {
        match topic {
            Topic::Client { id: target_client_id } => {
                // Direct message to a specific client
                if let Some(client_connection_stream) = state.get_connection_stream_by_client_id(&target_client_id) {
                    // Wrap payload in Broadcast message for client
                    let broadcast_msg = NetworkMessage::Broadcast {
                        payload_len: payload.len() as u16,
                        payload,
                    };
                    
                    if let Ok(packet) = encode_message(&broadcast_msg) {
                        if let Err(e) = peer.send(
                            &client_connection_stream.connection,
                            &client_connection_stream.stream,
                            Bytes::from(packet)
                        ) {
                            tracing::warn!(
                                "Failed to relay message to client {}: {}",
                                target_client_id.0, e
                            );
                        } else {
                            tracing::debug!("Relayed message to client {}", target_client_id.0);
                        }
                    }
                } else {
                    tracing::debug!("Cannot relay to client {}: not connected", target_client_id.0);
                }
            }
            _ => {
                // ShardInstance or other topics - use old behavior
                relay_shard_to_subscribers(peer, state, connection, stream, data);
            }
        }
    } else {
        // Not a Publish message - use old behavior
        relay_shard_to_subscribers(peer, state, connection, stream, data);
    }
}

fn relay_shard_to_subscribers(
    peer: &GamePeer,
    state: &mut PubSubState,
    connection: &GameConnection,
    stream: &GameStream,
    data: &[u8],
) {
    let connection_stream = ConnectionStream{
        connection: *connection,
        stream: stream.clone(),
    };

    let shard_topic = match state
        .get_shard_by_connection_stream(&connection_stream)
        .copied()
    {
        Some(shard_topic_found) => shard_topic_found,
        None => {
            tracing::debug!(
                "cannot publish : no shard registered for connection stream {:?}",
                connection_stream.connection.connection_id
            );
            return;
        }
    };


    let Some(subscribers) =
        state.topic_subscribers.get(&shard_topic)
    else {
        tracing::debug!(
            "cannot publish : no subscribers for topic {}",
            &shard_topic.to_string()
        );
        return;
    };

    let packet = Vec::from(data);

   for client_id in subscribers.iter() {
       let Some(connection_stream) =
           state.client_connections.get_by_left(&client_id)
       else {
           tracing::debug!(
               "cannot publish to client {}: no client connection registered",
               client_id.0
           );
           return;
       };

       if let Err(error) =
           peer.send(
               &connection_stream.connection,
               &connection_stream.stream,
               Bytes::from(packet.clone()))
       {
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
    state: &mut PubSubState,
    connection: &GameConnection,
    stream: &GameStream,
    data: &[u8],
) {
    let connection_stream = ConnectionStream{
        connection : connection.clone(),
        stream: stream.clone(),
    };

    let Some(client_id) = state.get_client_id_by_connection_stream(&connection_stream) else {
        tracing::warn!(
            "cannot relay input: no client known for connection {}",
            connection.connection_id
        );
        return;
    };

    let topic = match state.input_topic_for_client(client_id) {
        Some(found_topic) => found_topic,
        None => {
            tracing::warn!(
                "cannot relay input: no topic known for client {}",
                client_id.0
            );
            return;
        }
    };

    let shard_connection = match state.shard_streams_by_topic.get_by_left(&topic){
        Some(connection) => connection,
        None => {
            tracing::warn!("no shard connection found for topic: {:?}", topic);
            return;
        }
    };

    let packet = Vec::from(data);


    if let Err(error) =
        peer.send(
            &shard_connection.connection,
            &shard_connection.stream,
            Bytes::from(packet)) {
        tracing::warn!(
            "failed to relay input from client {} to shard topic {}: {}",
            client_id.0,
            &topic.to_string(),
            error
        );
    }
}


pub fn relay_to_spatial_services(
    peer: &GamePeer,
    state: &mut PubSubState,
    data: &[u8],
) {
    let packet = data.to_vec();
    if let Err(error) =
        state.spatial_handle.send_to_spatial(peer, packet.into()){
        tracing::warn!(
            "failed to send PositionUpdate to spatial service: {}",
            error
        );
    }
}

pub fn relay_handoff_request_to_shards(
    peer: &GamePeer,
    state: &mut PubSubState,
    connection: &GameConnection,
    stream: &GameStream,
    data: &[u8],
    ){

    let message = match decode_message(data) {
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
        NetworkMessage::HandoffRequest{entity_id,..} => {
            let Some(topic) = state.get_ghost_entity_destination(&entity_id) else {
                tracing::warn!(
                    "no destination for {}:",
                    entity_id.0
                );
                return;
            };

            let Some(shard_connection) = state.get_connection_stream_by_shard(topic)else{
                tracing::warn!(
                "could not find shard connection for topic: {}",
                topic.to_string()
                );
                return;
            };

            let packet = data.to_vec();

            if let Err(error) = peer.send(&shard_connection.connection, &shard_connection.stream, packet.into()){
                tracing::error!("failed to send packet to shard: {error:#}");
                return;
            }

        }
        _ => {
            tracing::warn!(
                "invalid message HandoffStart sent by{}:",
                connection.connection_id
            );
        }

    };
}

pub fn relay_handoff_completed_to_shard(
    peer: &GamePeer,
    state: &mut PubSubState,
    connection: &GameConnection,
    stream: &GameStream,
    data: &[u8],
) {

    let message = match decode_message(data) {
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
        NetworkMessage::HandoffCompleted{entity_id,..} => {
            let Some(topic) = state.get_ghost_entity_destination(&entity_id) else {
                tracing::warn!(
                    "no destination for HandoffCompleted entity {}:",
                    entity_id.0
                );
                return;
            };

            let Some(shard_connection) = state.get_connection_stream_by_shard(topic)else{
                tracing::warn!(
                "could not find shard connection for topic: {}",
                topic.to_string()
                );
                return;
            };

            let packet = data.to_vec();

            if let Err(error) = peer.send(&shard_connection.connection, &shard_connection.stream, Bytes::from(packet.clone())){
                tracing::error!("failed to send HandoffCompleted to dest shard: {error:#}");
                return;
            }

            // Also notify spatial so it can update entity shard + subscription.
            if let Err(error) = state.spatial_handle.send_to_spatial(peer, packet){
                tracing::warn!("failed to CC HandoffCompleted to spatial: {error:#}");
            }

            // Ghost route no longer needed.
            state.remove_ghost_entity(entity_id);
        }
        _ => {
            tracing::warn!(
                "invalid message HandoffCompleted sent by{}:",
                connection.connection_id
            );
        }

    };
}

pub fn relay_handoff_accepted_to_shard(
    peer: &GamePeer,
    state: &mut PubSubState,
    connection: &GameConnection,
    stream: &GameStream,
    data: &[u8],
) {

    let message = match decode_message(data) {
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
        NetworkMessage::HandoffAccepted{entity_id,..} => {
            let Some(topic) = state.get_ghost_entity_source(&entity_id) else {
                tracing::warn!(
                    "no source for HandoffAccepted entity {}:",
                    entity_id.0
                );
                return;
            };

            let Some(shard_connection) = state.get_connection_stream_by_shard(topic) else {
                tracing::warn!(
                "could not find source shard connection for entity: {}",
                entity_id.0
                );
                return;
            };

            let packet = data.to_vec();

            if let Err(error) = peer.send(&shard_connection.connection, &shard_connection.stream, packet.into()){
                tracing::error!("failed to send HandoffAccepted to source shard: {error:#}");
                return;
            }

        }
        _ => {
            tracing::warn!(
                "invalid message HandoffAccepted sent by{}:",
                connection.connection_id
            );
        }

    };
}

pub fn relay_handoff_rejected_to_shard(
    peer: &GamePeer,
    state: &mut PubSubState,
    connection: &GameConnection,
    stream: &GameStream,
    data: &[u8],
) {

    let message = match decode_message(data) {
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
        NetworkMessage::HandoffRejected{entity_id,..} => {
            let Some(topic) = state.get_ghost_entity_source(&entity_id) else {
                tracing::warn!(
                    "no source for HandoffRejected entity {}:",
                    entity_id.0
                );
                return;
            };

            let Some(shard_connection) = state.get_connection_stream_by_shard(topic) else {
                tracing::warn!(
                "could not find source shard connection for entity: {}",
                entity_id.0
                );
                return;
            };

            let packet = data.to_vec();

            if let Err(error) = peer.send(&shard_connection.connection, &shard_connection.stream, packet.into()){
                tracing::error!("failed to send HandoffRejected to source shard: {error:#}");
                return;
            }

        }
        _ => {
            tracing::warn!(
                "invalid message HandoffRejected sent by{}:",
                connection.connection_id
            );
        }

    };
}

pub fn relay_entity_id_block_allocated_to_shard(
    peer: &GamePeer,
    state: &mut PubSubState,
    connection: &GameConnection,
    stream: &GameStream,
    data: &[u8],
) {
    let Some(shard_id) = state.pop_entity_id_block_request() else {
        tracing::warn!("EntityIdBlockAllocated received but request queue is empty");
        return;
    };

    let topic = shared::protocol::Topic::ShardInstance { id: shard_id };
    let Some(shard_cs) = state.get_connection_stream_by_shard(topic) else {
        tracing::warn!(
            "EntityIdBlockAllocated: no connection for shard {}",
            shard_id.0
        );
        return;
    };

    let packet = data.to_vec();
    if let Err(e) = peer.send(&shard_cs.connection, &shard_cs.stream, packet.into()) {
        tracing::error!(
            "failed to relay EntityIdBlockAllocated to shard {}: {e:#}",
            shard_id.0
        );
    }
}

pub fn relay_ghost_update(
    peer: &GamePeer,
    state: &mut PubSubState,
    connection: &GameConnection,
    stream: &GameStream,
    data: &[u8],
){
    let message = match decode_message(data) {
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
        NetworkMessage::GhostUpdate{entity_id,..} => {
            let Some(topic) = state.get_ghost_entity_destination(&entity_id) else {
                tracing::warn!(
                    "no destination for {}:",
                    entity_id.0
                );
                return;
            };

            let Some(shard_connection) = state.get_connection_stream_by_shard(topic)else{
                tracing::warn!(
                "could not find shard connection for topic: {}",
                topic.to_string()
                );
                return;
            };

            let packet = data.to_vec();

            if let Err(error) = peer.send(&shard_connection.connection, &shard_connection.stream, packet.into()){
                tracing::error!("failed to send packet to shard: {error:#}");
                return;
            }

        }
        _ => {
            tracing::warn!(
                "invalid message Ghost update sent by{}:",
                connection.connection_id
            );
        }

    };

}

pub fn relay_handoff_start_to_shards(
peer: &GamePeer,
peer_roles: &mut PeerRoles,
state: &mut PubSubState,
connection: &GameConnection,
stream: &GameStream,
data: &[u8],
){

    if !peer_roles.ensure(*connection, PeerRole::SpatialService, "Subscribe") {
        tracing::warn!(
                "message not received from Spatial service {}:",
                connection.connection_id
            );
        return;
    }

    let message = match decode_message(data) {
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
        NetworkMessage::HandoffStart {
            entity_id, source,destination
        } => {
            let ghost_route = GhostRoute{
                source,
                destination
            };
            state.add_ghost_entity(entity_id, ghost_route);

            let Some(topic) =state.get_ghost_entity_source(&entity_id)else{
                tracing::warn!(
                "could not find source for ghost: {}",
                entity_id.0
                );
                return;
            };

            let Some(shard_connection) = state.get_connection_stream_by_shard(topic)else{
                tracing::warn!(
                "could not find shard connection for topic: {}",
                topic.to_string()
                );
                return;
            };

            let packet = data.to_vec();

            if let Err(error) = peer.send(&shard_connection.connection, &shard_connection.stream, packet.into()){
                tracing::error!("failed to send packet to shard: {error:#}");
                return;
            }

        }
        _ => {
            tracing::warn!(
                "invalid message HandoffStart sent by{}:",
                connection.connection_id
            );
        }
    };

}


