use crate::config::ServerConfig;
use crate::net::gameplay_message::handle_client_message;
use crate::net::login::remove_player_for_connection;
use crate::world::state::PlayerRegistry;
use bevy::prelude::*;
use bytes::Bytes;
use shared::game_sockets::protocols::QuicBackend;
use shared::game_sockets::{
    GameConnection, GameNetworkEvent, GamePeer, GameStream, GameStreamReliability,
};
use shared::protocol::transport::codec;
use shared::protocol::{ClientGameMessage, PlayerId, ServerGameMessage};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Resource, Clone)]
pub struct SharedPlayerRegistry {
    pub inner: Arc<Mutex<PlayerRegistry>>,
}

#[derive(Resource)]
pub struct GameplayPeer {
    pub peer: GamePeer,
    pub reliable_streams: HashMap<GameConnection, GameStream>,
    pub connection_players: HashMap<GameConnection, PlayerId>,
}

pub fn start_quic_server(mut commands: Commands, config: Res<ServerConfig>) {
    let peer = GamePeer::new(QuicBackend::new());

    peer.listen("0.0.0.0", config.port)
        .expect("failed to start game_sockets QUIC listener");

    tracing::info!(
        "game_sockets QUIC gameplay server listening on 0.0.0.0:{} zone={} max_players={}",
        config.port,
        config.zone,
        config.max_players
    );

    commands.insert_resource(GameplayPeer {
        peer,
        reliable_streams: HashMap::new(),
        connection_players: HashMap::new(),
    });
}

pub fn poll_network_events(
    config: Res<ServerConfig>,
    mut gameplay_peer: ResMut<GameplayPeer>,
    registry: Res<SharedPlayerRegistry>,
) {
    loop {
        let event = match gameplay_peer.peer.poll() {
            Ok(Some(event)) => event,
            Ok(None) => break,
            Err(error) => {
                tracing::error!("failed to poll gameplay peer: {error}");
                break;
            }
        };

        handle_network_event(&config, &mut gameplay_peer, &registry, event);
    }
}

fn handle_network_event(
    config: &ServerConfig,
    gameplay_peer: &mut GameplayPeer,
    registry: &SharedPlayerRegistry,
    event: GameNetworkEvent,
) {
    match event {
        GameNetworkEvent::Connected(connection) => {
            tracing::info!("client connected: {}", connection.connection_id);

            if let Err(error) = gameplay_peer
                .peer
                .create_stream(connection, GameStreamReliability::Reliable)
            {
                tracing::error!(
                    "failed to create reliable stream for {}: {}",
                    connection.connection_id,
                    error
                );
            }
        }

        GameNetworkEvent::Disconnected(connection) => {
            tracing::info!("client disconnected: {}", connection.connection_id);

            gameplay_peer.reliable_streams.remove(&connection);
            remove_player_for_connection(gameplay_peer, registry, connection);
        }

        GameNetworkEvent::StreamCreated(connection, stream) => {
            tracing::info!(
                "stream created: connection={} stream={}",
                connection.connection_id,
                stream.stream_id
            );

            if stream.is_reliable() {
                gameplay_peer.reliable_streams.insert(connection, stream);
            }
        }

        GameNetworkEvent::StreamClosed(connection, stream) => {
            tracing::info!(
                "stream closed: connection={} stream={}",
                connection.connection_id,
                stream.stream_id
            );

            gameplay_peer.reliable_streams.remove(&connection);
        }

        GameNetworkEvent::Message {
            connection,
            stream,
            data,
        } => {
            handle_network_message(config, gameplay_peer, registry, connection, stream, data);
        }

        GameNetworkEvent::Error { connection, inner } => {
            tracing::warn!(
                "game socket error on connection {}: {}",
                connection.connection_id,
                inner
            );
        }
    }
}

fn handle_network_message(
    config: &ServerConfig,
    gameplay_peer: &mut GameplayPeer,
    registry: &SharedPlayerRegistry,
    connection: GameConnection,
    stream: GameStream,
    data: Bytes,
) {
    let request = match codec::decode::<ClientGameMessage>(&data) {
        Ok(request) => request,
        Err(error) => {
            tracing::warn!(
                "failed to decode ClientGameMessage from {}: {error:#}",
                connection.connection_id
            );

            let response = ServerGameMessage::JoinRejected {
                reason: "invalid_message".to_string(),
            };

            send_response(gameplay_peer, connection, stream, &response);
            return;
        }
    };

    let response = handle_client_message(config, registry, gameplay_peer, connection, request);

    send_response(gameplay_peer, connection, stream, &response);
}

fn send_response(
    gameplay_peer: &GameplayPeer,
    connection: GameConnection,
    stream: GameStream,
    response: &ServerGameMessage,
) {
    let payload = match codec::encode(response) {
        Ok(payload) => payload,
        Err(error) => {
            tracing::error!("failed to encode ServerGameMessage: {error:#}");
            return;
        }
    };

    if let Err(error) = gameplay_peer.peer.send(&connection, &stream, payload.into()) {
        tracing::error!(
            "failed to send response to player {}: {}",
            connection.connection_id,
            error
        );
    }
}