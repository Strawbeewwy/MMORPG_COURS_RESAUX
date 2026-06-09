use crate::pubsub::state::{ConnectionStream, PubSubState};
use bytes::Bytes;
use game_sockets::{
    GameConnection, GameStream, GamePeer
};

pub fn relay_to_client(
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

    let spatial_connection = match state.spatial_service_streams.clone(){
        Some(spatial_connection) => spatial_connection,
        None => {
            tracing::warn!(
                " no spatial service registered",
            );
            return;
        }
    };

    let packet = Vec::from(data);

    if let Err(error) =
        peer.send(
            &spatial_connection.connection,
            &spatial_connection.stream,
            Bytes::from(packet)) {
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

   //TODO
}

pub fn relay_handoff_completed_to_shard(
    peer: &GamePeer,
    state: &mut PubSubState,
    connection: &GameConnection,
    stream: &GameStream,
    data: &[u8],
) {

 //TODO
}

pub fn relay_entity_id_block_allocated_to_shard(
    peer: &GamePeer,
    state: &mut PubSubState,
    connection: &GameConnection,
    stream: &GameStream,
    data: &[u8],
) {
    //TODO
}

pub fn relay_ghost_update(
    peer: &GamePeer,
    state: &mut PubSubState,
    connection: &GameConnection,
    stream: &GameStream,
    data: &[u8],
){
//TODO

}


