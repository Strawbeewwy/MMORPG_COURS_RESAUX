use bevy::prelude::*;
use game_sockets::{GameNetworkEvent, GameStreamReliability};
use shared::protocol::spatial::PositionUpdate as WirePositionUpdate;
use crate::messages::PositionUpdateMsg;
use crate::resources::client_map::ClientMap;
use crate::resources::net_handles::{BrokerClient, BrokerConnectionState, ShardListener};

/// Poll the shard listener peer each frame (non-blocking).
/// Decoded PositionUpdate wire packets are forwarded as Bevy messages.
/// Clients are removed from ClientMap on shard disconnect to prevent memory leaks.
pub fn poll_shard_events(
    mut listener: ResMut<ShardListener>,
    mut client_map: ResMut<ClientMap>,
    mut ev_writer: MessageWriter<PositionUpdateMsg>,
) {
    loop {
        match listener.peer.poll() {
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
            if let Err(e) = listener.peer.create_stream(conn, GameStreamReliability::Reliable) {
                tracing::error!("failed to create stream for shard {}: {e}", conn.connection_id);
            }
        }
        Disconnected(conn) => {
            tracing::info!("shard disconnected: {}", conn.connection_id);
            listener.streams.remove(&conn);
            // Remove all clients that were tracked via this shard connection
            // to prevent unbounded growth of ClientMap.
            client_map.remove_by_connection(conn);
        }
        StreamCreated(conn, stream) => {
            listener.streams.insert(conn, stream);
        }
        StreamClosed(conn, _stream) => {
            listener.streams.remove(&conn);
        }
        Message { data, .. } => {
            match WirePositionUpdate::from_bytes(&data) {
                Ok(u) => {
                    ev_writer.write(PositionUpdateMsg {
                        client_id: u.client_id,
                        x: u.x as f64,
                        y: u.y as f64,
                    });
                }
                Err(e) => tracing::warn!("invalid PositionUpdate from shard: {e}"),
            }
        }
        Error { connection, inner } => {
            tracing::warn!("shard socket error on {}: {inner}", connection.connection_id);
        }
    }
}

/// Poll the broker peer to advance handshake state and maintain the connection.
pub fn poll_broker_connection(mut broker: ResMut<BrokerClient>) {
    loop {
        match broker.peer.poll() {
            Ok(Some(event)) => handle_broker_event(&mut broker, event),
            Ok(None) => break,
            Err(e) => {
                tracing::error!("broker client poll error: {e}");
                break;
            }
        }
    }
}

fn handle_broker_event(broker: &mut BrokerClient, event: GameNetworkEvent) {
    use game_sockets::GameNetworkEvent::*;
    match event {
        Connected(conn) => {
            tracing::info!("connected to broker: {}", conn.connection_id);
            broker.connection = Some(conn);
            broker.state = BrokerConnectionState::Connected;
            if let Err(e) = broker.peer.create_stream(conn, GameStreamReliability::Reliable) {
                tracing::error!("failed to create stream towards broker: {e}");
            }
        }
        Disconnected(_conn) => {
            tracing::warn!("broker connection lost — will reconnect next tick");
            broker.connection = None;
            broker.stream = None;
            broker.state = BrokerConnectionState::Disconnected;
        }
        StreamCreated(_conn, stream) => {
            tracing::info!("broker stream ready");
            broker.stream = Some(stream);
            broker.state = BrokerConnectionState::Ready;
        }
        StreamClosed(_conn, _stream) => {
            broker.stream = None;
            broker.state = BrokerConnectionState::Disconnected;
        }
        Error { connection, inner } => {
            tracing::warn!("broker error on {}: {inner}", connection.connection_id);
            broker.state = BrokerConnectionState::Disconnected;
        }
        _ => {}
    }
}



