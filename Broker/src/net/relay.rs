use crate::pubsub::state::PubSubState;
use bytes::Bytes;
use shared::game_sockets::{GamePeer};
use shared::protocol::{CLIENT_INPUT_LEN, ClientId, Topic, encode_message, NetworkMessage, EntityId, ShardId};
use shared::protocol::game::EntityState;
use shared::protocol::NetVec2;

pub fn relay_to_client(
    peer: &GamePeer,
    state: &PubSubState,
    topic: Topic,
    payload_len: u16,
    payload: &[u8],
) {
    let Some(subscribers) = state.topic_subscribers.get(&topic) else {
        tracing::debug!(
            "cannot publish : no subscribers for topic {}",
            &topic.to_string()
        );
        return;
    };

   for client_id in subscribers.iter() {
       let Some((connection, stream)) = state.client_connections.get(&client_id) else {
           tracing::debug!(
               "cannot publish to client {}: no client connection registered",
               client_id.0
           );
           return;
       };


       let packet = match encode_message(&NetworkMessage::Broadcast {
           payload_len,
           payload: Vec::from(payload),
       }) {
           Ok(packet) => packet,
           Err(error) => {
               tracing::warn!(
                   "cannot encode targeted broadcast for client {}: {error}",
                   client_id.0
               );
               return;
           }
       };

       if let Err(error) = peer.send(connection, stream, Bytes::from(packet)) {
           tracing::warn!(
               "failed to send targeted broadcast to client {} on connection {}: {}",
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

    let packet = match encode_message(&NetworkMessage::ClientInput {
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
    peer: &GamePeer,
    state: &PubSubState,
    entity_id: EntityId,
    position: NetVec2,
) {
    let (connection, stream) = match state.spatial_service_streams.clone() {
        Some((connection, stream)) => (connection, stream),
        None => {
            tracing::warn!(
                "cannot forward PositionUpdate for entity {}: no spatial service registered",
                entity_id.0
            );
            return;
        }
    };

    let packet = match encode_message(&NetworkMessage::PositionUpdate {
        entity_id,
        position,
    }) {
        Ok(packet) => packet,
        Err(error) => {
            tracing::warn!(
                "failed to encode PositionUpdate for entity {}: {}",
                entity_id.0,
                error
            );
            return;
        }
    };

    if let Err(error) = peer.send(&connection, &stream, Bytes::from(packet)) {
        tracing::warn!(
            "failed to send PositionUpdate entity_id={} position=({}, {}) to spatial service: {}",
            entity_id.0,
            position.x,
            position.y,
            error
        );
    }
}

pub fn relay_handoff_request_to_shards(
    peer : &GamePeer,
    state: &mut PubSubState,
    entity_id: EntityId,
    from_shard_id: ShardId,
    to_shard_id: ShardId,
    position: NetVec2,
    velocity: NetVec2,
    entity_state: EntityState,
    ){

    let (shard_connection, shard_stream) =  match state.get_shard_connection_and_stream(to_shard_id) {
        Some(connection) => connection,
        None => return,
    };



    let packet = match encode_message(&NetworkMessage::HandoffRequest {
        entity_id,
        from_shard_id,
        to_shard_id,
        position,
        velocity,
        entity_state,
    }) {
        Ok(packet) => packet,
        Err(error) => {
            tracing::warn!(
                        "failed to encode handoff request from shard {}: {}",
                        from_shard_id.0,
                        error
                    );
            return;
        }
    };


    if let Err(error) = peer.send(&shard_connection,&shard_stream,Bytes::from(packet)) {
        tracing::warn!(
            "failed to forward HandoffRequest for entity {} from shard {} to shard {} : {}",
            entity_id.0,
            from_shard_id.0,
            to_shard_id.0,
            error
        );
    }
}


pub fn relay_handoff_accepted_to_spatial(
    peer: &GamePeer,
    state: &mut PubSubState,
    entity_id: EntityId) {

    let (connection, stream) = match state.spatial_service_streams.clone() {
        Some((connection, stream)) => (connection, stream),
        None => {
            tracing::warn!(
                    "cannot forward handoff accept for entity {}: no spatial service registered",
                    entity_id.0
                );
            return;
        }
    };

    let packet = match encode_message(&NetworkMessage::HandoffAccepted {
        entity_id,
    }) {
        Ok(packet) => packet,
        Err(error) => {
            tracing::warn!(
                        "failed to encode Handoff accept for entity {}: {}",
                        entity_id.0,
                        error
                    );
            return;
        }
    };

    if let Err(error) = peer.send(&connection,&stream,Bytes::from(packet)) {
        tracing::warn!(
            "failed to forward HandoffAccepted for entity {} : {}",
            entity_id.0,
            error
        );
    }

}

pub fn relay_handoff_rejected_to_spatial(
    peer: &GamePeer,
    state: &mut PubSubState,
    entity_id: EntityId) {

    let (connection, stream) = match state.spatial_service_streams.clone() {
        Some((connection, stream)) => (connection, stream),
        None => {
            tracing::warn!(
                    "cannot forward handoff reject for entity {}: no spatial service registered",
                    entity_id.0
                );
            return;
        }
    };

    let packet = match encode_message(&NetworkMessage::HandoffRejected {
        entity_id,
    }) {
        Ok(packet) => packet,
        Err(error) => {
            tracing::warn!(
                        "failed to encode Handoff reject for entity {}: {}",
                        entity_id.0,
                        error
                    );
            return;
        }
    };

    if let Err(error) = peer.send(&connection,&stream,Bytes::from(packet)) {
        tracing::warn!(
            "failed to forward HandoffRejected for entity {} : {}",
            entity_id.0,
            error
        );
    }

}

pub fn relay_handoff_completed_to_shard(
    peer: &GamePeer,
    state: &mut PubSubState,
    entity_id: EntityId,
) {

    let to_shard_id = state.ghost_entity.get(&entity_id).unwrap().1;


    let (shard_connection, shard_stream) =  match state.get_shard_connection_and_stream(to_shard_id) {
        Some((connection,stream)) => (connection,stream),
        None => return,
    };
    
    
    let packet = match encode_message(&NetworkMessage::HandoffCompleted {
        entity_id,
    }) {
        Ok(packet) => packet,
        Err(error) => {
            tracing::warn!(
                        "failed to encode Handoff reject for entity {}: {}",
                        entity_id.0,
                        error
                    );
            return;
        }
    };
    

    if let Err(error) = peer.send(&shard_connection,&shard_stream,Bytes::from(packet)) {
        tracing::warn!(
            "failed to forward HandoffRejected for entity {} : {}",
            entity_id.0,
            error
        );
    }

}

pub fn relay_entity_id_block_request_to_spatial(
    peer: &GamePeer,
    state: &PubSubState,
    shard_id: ShardId,
    count: u32,
) {
    let (connection, stream) = match state.spatial_service_streams.clone() {
        Some((connection, stream)) => (connection, stream),
        None => {
            tracing::warn!(
                "cannot request EntityId block for shard {}: no spatial service registered",
                shard_id.0
            );
            return;
        }
    };

    let packet = match encode_message(&NetworkMessage::RequestEntityIdBlock {
        shard_id,
        count,
    }) {
        Ok(packet) => packet,
        Err(error) => {
            tracing::warn!(
                "failed to encode RequestEntityIdBlock shard_id={} count={}: {}",
                shard_id.0,
                count,
                error
            );
            return;
        }
    };

    if let Err(error) = peer.send(&connection, &stream, Bytes::from(packet)) {
        tracing::warn!(
            "failed to send RequestEntityIdBlock shard_id={} count={} to spatial: {}",
            shard_id.0,
            count,
            error
        );
    }
}

pub fn relay_entity_id_block_allocated_to_shard(
    peer: &GamePeer,
    state: &PubSubState,
    shard_id: ShardId,
    start: u32,
    count: u32,
) {
    let topic = shared::protocol::Topic::ShardInstance(shard_id);

    let (connection, stream) = match state.shard_streams_by_topic.get(&topic) {
        Some((connection, stream)) => (*connection, *stream),
        None => {
            tracing::warn!(
                "cannot forward EntityIdBlockAllocated: no shard connection for shard_id={}",
                shard_id.0
            );
            return;
        }
    };

    let packet = match encode_message(&NetworkMessage::EntityIdBlockAllocated {
        shard_id,
        start,
        count,
    }) {
        Ok(packet) => packet,
        Err(error) => {
            tracing::warn!(
                "failed to encode EntityIdBlockAllocated shard_id={} start={} count={}: {}",
                shard_id.0,
                start,
                count,
                error
            );
            return;
        }
    };

    if let Err(error) = peer.send(&connection, &stream, Bytes::from(packet)) {
        tracing::warn!(
            "failed to send EntityIdBlockAllocated to shard_id={} start={} count={}: {}",
            shard_id.0,
            start,
            count,
            error
        );
    }
}


