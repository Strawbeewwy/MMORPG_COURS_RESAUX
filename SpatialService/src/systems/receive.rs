use bevy::prelude::*;
use game_sockets::{GameConnection, GameNetworkEvent, GameStreamReliability};
use shared::protocol::{decode_message, NetworkMessage};
use crate::messages::PositionUpdateMsg;
use crate::resources::client_map::ClientMap;
use crate::resources::net_handles::{BrokerClient, BrokerConnectionState, ShardListener};

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

fn handle_shard_event(
    listener: &mut ShardListener,
    client_map: &mut ClientMap,
    ev_writer: &mut MessageWriter<PositionUpdateMsg>,
    event: GameNetworkEvent,
) {
    use game_sockets::GameNetworkEvent::*;
    match event {
        Connected(conn) => {
            tracing::info!("shard connected: {}", conn.connection_id);
            if let Err(e) = listener.handle.peer.create_stream(conn, GameStreamReliability::Reliable) {
                tracing::error!("failed to create stream for shard {}: {e}", conn.connection_id);
            }
        }
        Disconnected(conn) => {
            tracing::info!("shard disconnected: {}", conn.connection_id);
            listener.handle.unregister_shard(conn);
            // Remove all clients that were tracked via this shard connection
            // to prevent unbounded growth of ClientMap.
            client_map.remove_by_connection(conn);
        }
        StreamCreated(conn, stream) => {
            listener.handle.streams.insert(conn, stream);
        }
        StreamClosed(conn, _stream) => {
            listener.handle.streams.remove(&conn);
        }
        Error { connection, inner } => {
            tracing::warn!("shard socket error on {}: {inner}", connection.connection_id);
        }
        Message { connection, data, .. } => {
            handle_shard_message(listener, client_map, ev_writer, connection, &data);
        }
    }
}

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
        NetworkMessage::PositionUpdate { client_id, position } => {
            ev_writer.write(PositionUpdateMsg {
                client_id,
                shard_connection: Some(connection),
                // f32 → f64 widening: lossless, intentional (see PositionUpdateMsg doc).
                x: f64::from(position.x),
                y: f64::from(position.y),
            });
        }

        // Destination shard accepted the client — clear the pending handoff state.
        NetworkMessage::HandoffCompleted { entity_id } => {
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
    mut ev_writer: MessageWriter<PositionUpdateMsg>,
) {
    loop {
        match broker.handle.peer.poll() {
            Ok(Some(event)) => handle_broker_event(&mut broker, event, &mut ev_writer),
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
) {
    use game_sockets::GameNetworkEvent::*;
    match event {
        Connected(conn) => {
            tracing::info!("connected to utils: {}", conn.connection_id);
            broker.handle.connection = Some(conn);
            broker.handle.state = BrokerConnectionState::Connected;
            if let Err(e) = broker.handle.peer.create_stream(conn, GameStreamReliability::Reliable) {
                tracing::error!("failed to create stream towards utils: {e}");
            }
        }
        Disconnected(_conn) => {
            tracing::warn!("utils connection lost — will reconnect next tick");
            broker.handle.connection = None;
            broker.handle.stream = None;
            broker.handle.state = BrokerConnectionState::Disconnected;
        }
        StreamCreated(_conn, stream) => {
            tracing::info!("utils stream ready");
            broker.handle.stream = Some(stream);
            broker.handle.state = BrokerConnectionState::Ready;
            broker.handle.reset_backoff();
        }
        StreamClosed(_conn, _stream) => {
            broker.handle.stream = None;
            broker.handle.state = BrokerConnectionState::Disconnected;
        }
        Error { connection, inner } => {
            tracing::warn!("utils error on {}: {inner}", connection.connection_id);
            broker.handle.state = BrokerConnectionState::Disconnected;
        }
        Message { connection, data, .. } => {
            handle_broker_message(connection, &data, ev_writer);
        }
    }
}

/// Handle a message received via the utils relay path.
/// PositionUpdates here have no direct shard connection — shard_connection is None.
pub fn handle_broker_message(
    connection: GameConnection,
    data: &[u8],
    ev_writer: &mut MessageWriter<PositionUpdateMsg>,
) {
    let message = match decode_message(data) {
        Ok(msg) => msg,
        Err(e) => {
            tracing::warn!("invalid broker message from connection {}: {e}", connection.connection_id);
            return;
        }
    };

    match message {
        NetworkMessage::PositionUpdate { client_id, position } => {
            ev_writer.write(PositionUpdateMsg {
                client_id,
                // Relayed via broker — no direct shard connection available.
                shard_connection: None,
                x: f64::from(position.x),
                y: f64::from(position.y),
            });
        }
        _ => {}
    }
}
