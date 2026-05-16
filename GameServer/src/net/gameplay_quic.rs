use crate::config::ServerConfig;
use crate::world::player::PlayerInfo;
use crate::world::state::PlayerRegistry;
use bevy::prelude::*;
use bytes::Bytes;
use shared::protocol::{
    ClientGameMessage, NetVec2, ServerGameMessage,
};
use shared::protocol::transport::codec;
use shared::protocol::transport::game_sockets_quic::QuicBackend;
use shared::protocol::transport::gamesockets_lib::{
    GameConnection, GameNetworkEvent, GamePeer, GameSocketError, GameStream,
    GameStreamReliability,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

const GAME_RELIABLE_STREAM_ID: u16 = 1;

#[derive(Resource, Clone)]
pub struct SharedPlayerRegistry {
    pub inner: Arc<Mutex<PlayerRegistry>>,
}

#[derive(Resource)]
pub struct GameplayPeer {
    pub peer: GamePeer,
    pub reliable_streams: HashMap<GameConnection, GameStream>,
}

pub fn start_gameplay_quic_server(mut commands: Commands, config: Res<ServerConfig>) {
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
    });
}

pub fn poll_gameplay_events(
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

        handle_gameplay_event(&config, &mut gameplay_peer, &registry, event);
    }
}

fn handle_gameplay_event(
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
            handle_gameplay_message(
                config,
                gameplay_peer,
                registry,
                connection,
                stream,
                data,
            );
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

fn handle_gameplay_message(
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

    let response = handle_client_message(config, registry, connection, request);

    send_response(gameplay_peer, connection, stream, &response);
}

fn handle_client_message(
    config: &ServerConfig,
    registry: &SharedPlayerRegistry,
    connection: GameConnection,
    message: ClientGameMessage,
) -> ServerGameMessage {
    match message {
        ClientGameMessage::JoinGame {
            protocol_version,
            session_token,
            username,
        } => handle_join_game(
            config,
            registry,
            connection,
            protocol_version,
            session_token,
            username,
        ),
        ClientGameMessage::LeaveGame => handle_leave_game(registry, connection),
        ClientGameMessage::Heartbeat => ServerGameMessage::HeartbeatAck,
        ClientGameMessage::PlayerInput {
            movement_x,
            movement_y,
        } => handle_player_input(registry, connection, movement_x, movement_y),
    }
}

fn handle_join_game(
    config: &ServerConfig,
    registry: &SharedPlayerRegistry,
    connection: GameConnection,
    protocol_version: u16,
    session_token: String,
    username: String,
) -> ServerGameMessage {
    const SUPPORTED_PROTOCOL_VERSION: u16 = 1;

    if protocol_version != SUPPORTED_PROTOCOL_VERSION {
        return ServerGameMessage::JoinRejected {
            reason: format!(
                "unsupported_protocol_version: expected {}, got {}",
                SUPPORTED_PROTOCOL_VERSION, protocol_version
            ),
        };
    }

    if session_token.trim().is_empty() {
        return ServerGameMessage::JoinRejected {
            reason: "missing_session_token".to_string(),
        };
    }

    if username.trim().is_empty() {
        return ServerGameMessage::JoinRejected {
            reason: "empty_username".to_string(),
        };
    }

    let Ok(mut registry) = registry.inner.try_lock() else {
        return ServerGameMessage::JoinRejected {
            reason: "server_busy".to_string(),
        };
    };

    if let Some(existing_player) = registry.players.get(&connection.connection_id) {
        let snapshot = registry.snapshot(config.zone.clone());

        return ServerGameMessage::JoinAccepted {
            player_id: existing_player.player_id.clone(),
            player: existing_player.public_info(),
            snapshot,
            message: "already_joined".to_string(),
        };
    }

    if registry.is_full(config.max_players) {
        return ServerGameMessage::JoinRejected {
            reason: "server_full".to_string(),
        };
    }

    let player = PlayerInfo {
        player_id: Uuid::new_v4().to_string(),
        username: username.trim().to_string(),
        connection,
        zone: config.zone.clone(),
        position: NetVec2::ZERO,
        velocity: NetVec2::ZERO,
    };

    let player_id = player.player_id.clone();
    let public_player = player.public_info();

    registry.players.insert(connection.connection_id, player);

    let snapshot = registry.snapshot(config.zone.clone());

    tracing::info!(
        "player joined: player_id={} connection={} players={}/{}",
        player_id,
        connection.connection_id,
        registry.player_count(),
        config.max_players
    );

    ServerGameMessage::JoinAccepted {
        player_id,
        player: public_player,
        snapshot,
        message: "welcome".to_string(),
    }
}

fn handle_leave_game(
    registry: &SharedPlayerRegistry,
    connection: GameConnection,
) -> ServerGameMessage {
    if let Ok(mut registry) = registry.inner.try_lock() {
        if let Some(player) = registry.players.remove(&connection.connection_id) {
            tracing::info!(
                "player left: player_id={} connection={}",
                player.player_id,
                connection.connection_id
            );
        }
    }

    ServerGameMessage::Goodbye
}

fn handle_player_input(
    registry: &SharedPlayerRegistry,
    connection: GameConnection,
    movement_x: f32,
    movement_y: f32,
) -> ServerGameMessage {
    let Ok(mut registry) = registry.inner.try_lock() else {
        return ServerGameMessage::JoinRejected {
            reason: "server_busy".to_string(),
        };
    };

    let Some(player) = registry.players.get_mut(&connection.connection_id) else {
        return ServerGameMessage::JoinRejected {
            reason: "not_joined".to_string(),
        };
    };

    player.velocity = NetVec2::new(movement_x, movement_y);

    ServerGameMessage::InputAccepted {
        movement_x,
        movement_y,
    }
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
            "failed to send response to connection {}: {}",
            connection.connection_id,
            error
        );
    }
}