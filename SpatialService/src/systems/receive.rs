use bevy::prelude::*;
use shared::game_sockets::{
    GameConnection, GameNetworkEvent, GameStreamReliability
};
use shared::{NetVec2, ShardId};
use shared::protocol::{ClientId, Topic, decode_message, encode_message, NetworkMessage};
use crate::messages::PositionUpdateMsg;
use crate::resources::client_map::ClientMap;
use crate::resources::entity_map::{EntityMap, EntityTransferState, SpatialEntityRecord};
use crate::resources::net_handles::{BrokerClient, BrokerConnectionState, ShardListener};
use crate::resources::quad_tree::QuadTree;

#[deprecated]/// The spatial should not communicate directly with a shard
/// Poll the shard listener peer each frame (non-blocking).
/// Handles shard lifecycle events, ShardRegister identification, PositionUpdate and HandoffAck.
pub fn poll_shard_events(
    mut listener: ResMut<ShardListener>,
    mut client_map: ResMut<ClientMap>,
    mut ev_writer: MessageWriter<PositionUpdateMsg>,
) {
    loop {
        match listener.handle.peer.poll() {
            Ok(Some(event)) => handle_shard_event(&mut listener, &mut client_map, &mut ev_writer, event),
            Ok(None) => break,
            Err(e) => {
                tracing::error!("shard listener poll error: {e}");
                break;
            }
        }
    }
}
#[deprecated]/// The spatial should not communicate directly with a shard
fn handle_shard_event(
    listener: &mut ShardListener,
    client_map: &mut ClientMap,
    ev_writer: &mut MessageWriter<PositionUpdateMsg>,
    event: GameNetworkEvent,
) {
    match event {
        GameNetworkEvent::Connected(conn) => {
            tracing::info!("shard connected: {}", conn.connection_id);
            if let Err(e) = listener.handle.peer.create_stream(conn, GameStreamReliability::Reliable) {
                tracing::error!("failed to create stream for shard {}: {e}", conn.connection_id);
            }
        }
        GameNetworkEvent::Disconnected(conn) => {
            tracing::info!("shard disconnected: {}", conn.connection_id);
            listener.handle.unregister_shard(conn);
            // Remove all clients that were tracked via this shard connection
            // to prevent unbounded growth of ClientMap.
            client_map.remove_by_connection(conn);
        }
        GameNetworkEvent::StreamCreated(conn, stream) => {
            listener.handle.streams.insert(conn, stream);
        }
        GameNetworkEvent::StreamClosed(conn, _stream) => {
            listener.handle.streams.remove(&conn);
        }
        GameNetworkEvent::Error { connection, inner } => {
            tracing::warn!("shard socket error on {}: {inner}", connection.connection_id);
        }
        GameNetworkEvent::Message { connection, data, .. } => {
            handle_shard_message(listener, client_map, ev_writer, connection, &data);
        }
    }
}
#[deprecated]/// The spatial should not communicate directly with a shard
/// Decode and dispatch a message received directly from a connected shard.
fn handle_shard_message(
    listener: &mut ShardListener,
    client_map: &mut ClientMap,
    ev_writer: &mut MessageWriter<PositionUpdateMsg>,
    connection: GameConnection,
    data: &[u8],
) {
    let message = match decode_message(data) {
        Ok(msg) => msg,
        Err(e) => {
            tracing::warn!("invalid message from shard {}: {e}", connection.connection_id);
            return;
        }
    };

    match message {
        // Shard identifies itself — register the shard_id ↔ connection mapping.
        NetworkMessage::RegisterShard { shard_id } => {
            listener.handle.register_shard(connection, shard_id);
        }

        // Direct PositionUpdate from shard — propagate the source connection.
        NetworkMessage::PositionUpdate { entity_id, position } => {
            ev_writer.write(PositionUpdateMsg {
                entity_id,
                //shard_connection: Some(connection),
                // f32 → f64 widening: lossless, intentional (see PositionUpdateMsg doc).
                x: f64::from(position.x),
                y: f64::from(position.y),
            });
        }

        // Destination shard accepted the client — clear the pending handoff state.
        NetworkMessage::HandoffCompleted { entity_id,.. } => {
            tracing::info!(
                "HandoffAck received: client {}",
                entity_id.0,
            );
            //TODO we need an entity map that will manage all entities
            //we can then map entities to clients
            //client_map.clear_state(client_id.into());
        }

        other => {
            tracing::warn!(
                "unexpected message from shard {}: {:?}",
                connection.connection_id, other
            );
        }
    }
}

/// Poll the utils peer to advance handshake state and maintain the connection.
pub fn poll_broker_connection(
    mut broker: ResMut<BrokerClient>,
    mut entity_map: ResMut<EntityMap>,
    mut ev_writer: MessageWriter<PositionUpdateMsg>,
    mut quad_tree: ResMut<QuadTree>,
) {
    loop {
        match broker.handle.peer.poll() {
            Ok(Some(event)) => handle_broker_event(&mut broker, event, &mut ev_writer, &mut entity_map, &mut quad_tree),
            Ok(None) => break,
            Err(e) => {
                tracing::error!("utils client poll error: {e}");
                break;
            }
        }
    }
}

fn handle_broker_event(
    broker: &mut BrokerClient,
    event: GameNetworkEvent,
    ev_writer: &mut MessageWriter<PositionUpdateMsg>,
    entity_map: &mut EntityMap,
    quad_tree: &mut QuadTree,
) {
    match event {
        GameNetworkEvent::Connected(conn) => {
            tracing::info!("connected to utils: {}", conn.connection_id);
            broker.handle.connection = Some(conn);
            broker.handle.state = BrokerConnectionState::Connected;
            if let Err(e) = broker.handle.peer.create_stream(conn, GameStreamReliability::Reliable) {
                tracing::error!("failed to create stream towards utils: {e}");
            }
        }
        GameNetworkEvent::Disconnected(_conn) => {
            tracing::warn!("utils connection lost — will reconnect next tick");
            broker.handle.connection = None;
            broker.handle.stream = None;
            broker.handle.state = BrokerConnectionState::Disconnected;
        }
        GameNetworkEvent::StreamCreated(_conn, stream) => {
            tracing::info!("utils stream ready");
            broker.handle.stream = Some(stream);
            broker.handle.state = BrokerConnectionState::Ready;
            broker.handle.reset_backoff();
        }
        GameNetworkEvent::StreamClosed(_conn, _stream) => {
            broker.handle.stream = None;
            broker.handle.state = BrokerConnectionState::Disconnected;
        }
        GameNetworkEvent::Error { connection, inner } => {
            tracing::warn!("utils error on {}: {inner}", connection.connection_id);
            broker.handle.state = BrokerConnectionState::Disconnected;
        }
        GameNetworkEvent::Message { connection, data, .. } => {
            handle_broker_message(connection, &data, ev_writer, entity_map, quad_tree, broker);
        }
    }
}

/// Handle a message received via the utils relay path.
/// PositionUpdates here have no direct shard connection — shard_connection is None.
pub fn handle_broker_message(
    connection: GameConnection,
    data: &[u8],
    ev_writer: &mut MessageWriter<PositionUpdateMsg>,
    entity_map: &mut EntityMap,
    quad_tree: &mut QuadTree,
    broker: &mut BrokerClient,
) {
    let message = match decode_message(data) {
        Ok(msg) => msg,
        Err(e) => {
            tracing::warn!("invalid broker message from connection {}: {e}", connection.connection_id);
            return;
        }
    };

    match message {
        NetworkMessage::PositionUpdate { entity_id, position } => {
            ev_writer.write(PositionUpdateMsg {
                entity_id,
                x: f64::from(position.x),
                y: f64::from(position.y),
            });
        }
        NetworkMessage::RegisterEntity {entity_id, client_id, position} => {

            let f32_position= Vec2::from(NetVec2::to_f32(&position));

            let Some(shard) = quad_tree.shard_for(f32_position.x,f32_position.y) else {
                tracing::warn!("no shard for specified position : x{} y{}",
                    f32_position.x,
                    f32_position.y
                );
                return;
            };

            let record = SpatialEntityRecord {
                entity_id,
                client_id,
                position : f32_position,
                current_shard: shard,
            };
            entity_map.insert(entity_id, record);
        }
        NetworkMessage::UnregisterEntity {entity_id} => {
            entity_map.remove(entity_id);
        }
        NetworkMessage::HandoffCompleted { entity_id } => {
            // Broker CCs spatial on HandoffCompleted so it can update subscriptions.
            if let EntityTransferState::PendingHandoff { destination_shard } = entity_map.get_state(entity_id) {
                let dest_shard_id = ShardId(destination_shard);

                if let Some(record) = entity_map.entities.get_mut(&entity_id) {
                    if record.is_player() {
                        let client_id = record.client_id;
                        let old_shard = record.current_shard;

                        // Unsubscribe from source shard.
                        if let Ok(packet) = encode_message(&NetworkMessage::Unsubscribe {
                            client_id,
                            topic: Topic::ShardInstance { id: old_shard },
                        }) {
                            if let Err(e) = broker.handle.send(packet) {
                                tracing::error!("HandoffCompleted: failed unsubscribe: {e:#}");
                            }
                        }

                        // Subscribe to destination shard.
                        if let Ok(packet) = encode_message(&NetworkMessage::Subscribe {
                            client_id,
                            topic: Topic::ShardInstance { id: dest_shard_id },
                        }) {
                            if let Err(e) = broker.handle.send(packet) {
                                tracing::error!("HandoffCompleted: failed subscribe: {e:#}");
                            }
                        }
                    }
                    record.current_shard = dest_shard_id;
                }
                entity_map.clear_state(entity_id);

                tracing::info!(
                    "HandoffCompleted: entity {} moved to shard {}",
                    entity_id.0, destination_shard
                );
            }
        }
        _ => {}
    }
}
