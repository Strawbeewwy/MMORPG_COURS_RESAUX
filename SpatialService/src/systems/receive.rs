use bevy::prelude::*;
use game_sockets::{GameNetworkEvent, GameStreamReliability};
use shared::protocol::spatial::PositionUpdate as WirePositionUpdate;
use crate::messages::PositionUpdateMsg;
use crate::resources::net_handles::{BrokerClient, ShardListener};

/// Poll the shard listener and broker client peers each frame (non-blocking).
/// Mirrors the `poll_login_task` pattern: drains all pending events then stops.
///
/// - Shard connections are tracked in `ShardListener::streams`.
/// - Incoming PositionUpdate wire packets are decoded and forwarded as Bevy messages.
/// - Broker connection state is stored in `BrokerClient`.
pub fn poll_shard_events(
    mut listener: ResMut<ShardListener>,
    mut ev_writer: MessageWriter<PositionUpdateMsg>,
) {
    loop {
        match listener.peer.poll() {
            Ok(Some(event)) => handle_shard_event(&mut listener, &mut ev_writer, event),
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
        }
        StreamCreated(conn, stream) => {
            listener.streams.insert(conn, stream);
        }
        StreamClosed(conn, _stream) => {
            listener.streams.remove(&conn);
        }
        Message { data, .. } => {
            match WirePositionUpdate::from_bytes(&data) {
                Ok(u) => { ev_writer.write(PositionUpdateMsg { client_id: u.client_id, x: u.x, y: u.y }); }
                Err(e) => tracing::warn!("invalid PositionUpdate from shard: {e}"),
            }
        }
        Error { connection, inner } => {
            tracing::warn!("shard socket error on {}: {inner}", connection.connection_id);
        }
    }
}

/// Poll the broker peer to complete the outbound QUIC handshake and store the stream.
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
            if let Err(e) = broker.peer.create_stream(conn, GameStreamReliability::Reliable) {
                tracing::error!("failed to create stream towards broker: {e}");
            }
        }
        Disconnected(_conn) => {
            tracing::warn!("broker connection lost — Subscribe/Unsubscribe will be dropped");
            broker.connection = None;
            broker.stream = None;
        }
        StreamCreated(_conn, stream) => {
            tracing::info!("broker stream ready");
            broker.stream = Some(stream);
        }
        StreamClosed(_conn, _stream) => {
            broker.stream = None;
        }
        Error { connection, inner } => {
            tracing::warn!("broker error on {}: {inner}", connection.connection_id);
        }
        _ => {}
    }
}


