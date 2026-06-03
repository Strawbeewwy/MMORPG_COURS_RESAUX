use crate::pubsub::state::PubSubState;
use bytes::Bytes;
use shared::game_sockets::{GameConnection, GamePeer, GameStream};
use shared::protocol::{CLIENT_INPUT_LEN, ClientId, Topic, encode_message, NetworkMessage, WorldUpdate, WorldSnapshot};
use std::collections::HashMap;
use shared::protocol::NetVec2;
use shared::protocol::transport::codec;

const DEFAULT_AREA_OF_INTEREST_RADIUS: f32 = 25.0;

pub fn broadcast_to_subscribers(
    peer: &GamePeer,
    reliable_streams: &HashMap<GameConnection, GameStream>,
    state: &PubSubState,
    topic: Topic,
    payload_len: &u16,
    payload: &[u8],
) {
    if let Ok(WorldUpdate::Snapshot { snapshot }) = codec::decode::<WorldUpdate>(payload) {
        broadcast_aoi_snapshot_to_subscribers(
            peer,
            reliable_streams,
            state,
            topic,
            snapshot,
            DEFAULT_AREA_OF_INTEREST_RADIUS,
        );
        return;
    }

    let Some(subscribers) = state.topic_subscribers.get(&topic) else {
        return;
    };

    let packet = match encode_message(&NetworkMessage::Broadcast {
        payload_len: payload_len.clone(),
        payload: Vec::from(payload),
    }) {
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

        let Some(stream) = reliable_streams.get(connection) else {
            continue;
        };

        if let Err(error) = peer.send(connection, stream, Bytes::from(packet.clone())) {
            tracing::warn!(
                "failed to send broadcast to client {} on connection {}: {}",
                client_id.0,
                connection.connection_id,
                error
            );
        }
    }
}

fn broadcast_aoi_snapshot_to_subscribers(
    peer: &GamePeer,
    reliable_streams: &HashMap<GameConnection, GameStream>,
    state: &PubSubState,
    topic: Topic,
    snapshot: WorldSnapshot,
    radius: f32,
) {
    let Some(subscribers) = state.topic_subscribers.get(&topic) else {
        return;
    };

    for client_id in subscribers {
        let Some(connection) = state.client_connections.get(client_id) else {
            continue;
        };

        let Some(stream) = reliable_streams.get(connection) else {
            continue;
        };

        let Some(filtered_snapshot) = snapshot_for_client_from_snapshot(
            &snapshot,
            *client_id,
            radius,
        ) else {
            tracing::debug!(
                "cannot build AOI snapshot for client {} on topic {}: observer not found in snapshot",
                client_id.0,
                &topic.to_string()
            );
            continue;
        };

        let update = WorldUpdate::Snapshot {
            snapshot: filtered_snapshot,
        };

        let payload = match codec::encode(&update) {
            Ok(payload) => payload,
            Err(error) => {
                tracing::warn!(
                    "failed to encode AOI snapshot for client {}: {error:#}",
                    client_id.0
                );
                continue;
            }
        };

        let Ok(payload_len) = u16::try_from(payload.len()) else {
            tracing::warn!(
                "AOI snapshot payload for client {} is too large: {} bytes",
                client_id.0,
                payload.len()
            );
            continue;
        };

        let packet = match encode_message(&NetworkMessage::Broadcast {
            payload_len,
            payload,
        }) {
            Ok(packet) => packet,
            Err(error) => {
                tracing::warn!(
                    "cannot encode AOI broadcast for client {}: {error}",
                    client_id.0
                );
                continue;
            }
        };

        if let Err(error) = peer.send(connection, stream, Bytes::from(packet)) {
            tracing::warn!(
                "failed to send AOI snapshot to client {} on connection {}: {}",
                client_id.0,
                connection.connection_id,
                error
            );
        }
    }
}

fn snapshot_for_client_from_snapshot(
    snapshot: &WorldSnapshot,
    observer_client_id: ClientId,
    radius: f32,
) -> Option<WorldSnapshot> {
    let observer = snapshot
        .players
        .iter()
        .find(|player| player.client_id == observer_client_id)?;

    let players = snapshot
        .players
        .iter()
        .filter(|player| {
            player.client_id == observer_client_id
                || is_inside_area_of_interest(
                observer.position,
                player.position,
                radius,
            )
        })
        .cloned()
        .collect();

    Some(WorldSnapshot {
        zone: snapshot.zone.clone(),
        players,
        server_tick: snapshot.server_tick,
    })
}

fn is_inside_area_of_interest(
    observer_position: NetVec2,
    target_position: NetVec2,
    radius: f32,
) -> bool {
    distance_squared(observer_position, target_position) <= radius * radius
}

fn distance_squared(a: NetVec2, b: NetVec2) -> f32 {
    let a_f32 = a.to_f32();
    let b_f32 = b.to_f32();
    let dx = a_f32.0 - b_f32.0;
    let dy = a_f32.1 - b_f32.1;

    dx * dx + dy * dy
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
    peer : &GamePeer,
    state: &PubSubState,
    client_id: ClientId,
    position: NetVec2,
){

    if state.spatial_service_streams.is_empty() {
        tracing::warn!(
                    "cannot forward PositionUpdate for client {}: no spatial service registered",
                    client_id.0
                );
        return;
    }

    let packet = match encode_message(&NetworkMessage::PositionUpdate {
        client_id,
        position,
    }) {
        Ok(packet) => packet,
        Err(error) => {
            tracing::warn!(
                        "failed to encode PositionUpdate for client {}: {}",
                        client_id.0,
                        error
                    );
            return;
        }
    };

    for (spatial_connection, spatial_stream) in &state.spatial_service_streams {
        if let Err(error) = peer.send(
            spatial_connection,
            spatial_stream,
            Bytes::from(packet.clone()),
        ) {
            tracing::warn!(
                "failed to forward PositionUpdate for client {} to spatial service connection {}: {}",
                client_id.0,
                spatial_connection.connection_id,
                error
            );
        }
    }

    tracing::debug!(
        "forwarded PositionUpdate client_id={} position=({}, {}) to {} spatial service(s)",
        client_id.0,
        position.x,
        position.y,
        state.spatial_service_streams.len()
    );

}

