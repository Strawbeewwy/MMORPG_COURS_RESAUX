use crate::config::{ClientConfig, DEFAULT_RECONNECT_INTERVAL};
use crate::net::gameplay_message::handle_server_message;
use crate::net::login::send_join_game;
use crate::world::state::LocalWorldState;
use bevy::prelude::*;
use bytes::Bytes;
use shared::game_sockets::protocols::QuicBackend;
use shared::game_sockets::{
    GameConnection, GameNetworkEvent, GamePeer, GameStream, GameStreamReliability,
};
use shared::protocol::transport::codec;
use shared::protocol::{ClientGameMessage, ServerGameMessage};
use std::time::Duration;

#[derive(Resource)]
pub struct GameplayClient {
    pub peer: Option<GamePeer>,
    pub connection: Option<GameConnection>,
    pub reliable_stream: Option<GameStream>,
    pub joined: bool,
    pub reconnect_timer: Timer,
}

impl Default for GameplayClient {
    fn default() -> Self {
        Self {
            peer: None,
            connection: None,
            reliable_stream: None,
            joined: false,
            reconnect_timer: Timer::new(Duration::from_secs(DEFAULT_RECONNECT_INTERVAL), TimerMode::Repeating),
        }
    }
}
pub fn connect_to_game_server(
    config: Res<ClientConfig>,
    mut gameplay_client: ResMut<GameplayClient>,
    mut world_state: ResMut<LocalWorldState>,
) {
    tracing::info!(
        "starting GameClient for username={} player_id={} server={} zone={}",
        config.username,
        config.player_id,
        config.server_addr(),
        config.zone
    );

    world_state.player_id = Some(config.player_id.clone());
    world_state.zone = Some(config.zone.clone());

    try_connect_to_game_server(&config, &mut gameplay_client);
}
fn try_connect_to_game_server(
    config: &ClientConfig,
    gameplay_client: &mut GameplayClient,
) {
    tracing::info!(
        "trying to connect to game server {}:{}",
        config.server_ip,
        config.server_port
    );

    let peer = GamePeer::new(QuicBackend::new());

    if let Err(error) = peer.connect(&config.server_ip, config.server_port) {
        tracing::error!(
            "failed to start connection to game server {}:{}: {}",
            config.server_ip,
            config.server_port,
            error
        );
        return;
    }

    gameplay_client.peer = Some(peer);
    gameplay_client.connection = None;
    gameplay_client.reliable_stream = None;
    gameplay_client.joined = false;

    tracing::info!("connection attempt started to {}", config.server_addr());
}

pub fn retry_connection_if_needed(
    time: Res<Time>,
    config: Res<ClientConfig>,
    mut gameplay_client: ResMut<GameplayClient>,
) {
    if gameplay_client.joined || gameplay_client.connection.is_some() {
        return;
    }

    gameplay_client.reconnect_timer.tick(time.delta());

    if !gameplay_client.reconnect_timer.just_finished() {
        return;
    }

    tracing::info!(
        "not connected to game server yet; retrying {}",
        config.server_addr()
    );

    gameplay_client.peer = None;
    gameplay_client.connection = None;
    gameplay_client.reliable_stream = None;
    gameplay_client.joined = false;

    try_connect_to_game_server(&config, &mut gameplay_client);
}

pub fn poll_gameplay_events(
    config: Res<ClientConfig>,
    mut gameplay_client: ResMut<GameplayClient>,
    mut world_state: ResMut<LocalWorldState>,
) {
    loop {
        let event = {
            let Some(peer) = gameplay_client.peer.as_mut() else {
                return;
            };

            match peer.poll() {
                Ok(Some(event)) => event,
                Ok(None) => break,
                Err(error) => {
                    tracing::warn!(
                        "game peer poll failed while connecting to {}: {}",
                        config.server_addr(),
                        error
                    );

                    gameplay_client.peer = None;
                    gameplay_client.connection = None;
                    gameplay_client.reliable_stream = None;
                    gameplay_client.joined = false;

                    break;
                }
            }
        };

        handle_gameplay_event(&config, &mut gameplay_client, &mut world_state, event);
    }
}

fn handle_gameplay_event(
    config: &ClientConfig,
    gameplay_client: &mut GameplayClient,
    world_state: &mut LocalWorldState,
    event: GameNetworkEvent,
) {
    match event {
        GameNetworkEvent::Connected(connection) => {
            tracing::info!("connected to game server: {}", connection.connection_id);

            gameplay_client.connection = Some(connection);

            let Some(peer) = gameplay_client.peer.as_mut() else {
                return;
            };

            if let Err(error) = peer.create_stream(connection, GameStreamReliability::Reliable) {
                tracing::error!(
                    "failed to create reliable stream for {}: {}",
                    connection.connection_id,
                    error
                );
            }
        }

        GameNetworkEvent::Disconnected(connection) => {
            tracing::warn!("disconnected from game server: {}", connection.connection_id);

            gameplay_client.connection = None;
            gameplay_client.reliable_stream = None;
            gameplay_client.joined = false;
        }

        GameNetworkEvent::StreamCreated(connection, stream) => {
            tracing::info!(
                "stream created: connection={} stream={}",
                connection.connection_id,
                stream.stream_id
            );

            if stream.is_reliable() {
                gameplay_client.connection = Some(connection);
                gameplay_client.reliable_stream = Some(stream);

                if !gameplay_client.joined {
                    send_join_game(config, gameplay_client);
                }
            }
        }

        GameNetworkEvent::StreamCreated(connection, stream) => {
            tracing::info!(
                "stream created: connection={} stream={}",
                connection.connection_id,
                stream.stream_id
            );

            if stream.is_reliable() {
                gameplay_client.connection = Some(connection);
                gameplay_client.reliable_stream = Some(stream);

                if !gameplay_client.joined {
                    send_join_game(config, gameplay_client);
                }
            }
        }

        GameNetworkEvent::Message {
            connection,
            stream,
            data,
        } => {
            tracing::debug!(
                "message received: connection={} stream={} bytes={}",
                connection.connection_id,
                stream.stream_id,
                data.len()
            );

            decode_and_handle_server_message(gameplay_client, world_state, &data);
        }

        GameNetworkEvent::Error { connection, inner } => {
            tracing::warn!(
                "game socket error on connection {} while targeting {}: {}",
                connection.connection_id,
                config.server_addr(),
                inner
            );

            gameplay_client.peer = None;
            gameplay_client.connection = None;
            gameplay_client.reliable_stream = None;
            gameplay_client.joined = false;
        }
    }
}

fn decode_and_handle_server_message(
    gameplay_client: &mut GameplayClient,
    world_state: &mut LocalWorldState,
    data: &[u8],
) {
    let message = match codec::decode::<ServerGameMessage>(data) {
        Ok(message) => message,
        Err(error) => {
            tracing::warn!("failed to decode ServerGameMessage: {error:#}");
            return;
        }
    };

    handle_server_message(gameplay_client, world_state, message);
}

fn send_join_game(config: &ClientConfig, gameplay_client: &mut GameplayClient) {
    tracing::info!(
        "sending JoinGame username={} session_token={}",
        config.username,
        config.player_id
    );

    send_message(
        gameplay_client,
        ClientGameMessage::JoinGame {
            protocol_version: GAME_PROTOCOL_VERSION.to_string(),
            session_token: config.player_id.clone(),
            username: config.username.clone(),
        },
    );
}

pub fn send_player_input(
    gameplay_client: &mut GameplayClient,
    movement_x: f32,
    movement_y: f32,
) {
    send_message(
        gameplay_client,
        ClientGameMessage::PlayerInput {
            movement_x,
            movement_y,
        },
    );
}

pub fn send_message(
    gameplay_client: &mut GameplayClient,
    message: ClientGameMessage,
) {
    let Some(peer) = gameplay_client.peer.as_ref() else {
        tracing::warn!("cannot send message: peer is not ready");
        return;
    };

    let Some(connection) = gameplay_client.connection else {
        tracing::warn!("cannot send message: not connected yet");
        return;
    };

    let Some(stream) = gameplay_client.reliable_stream.clone() else {
        tracing::warn!("cannot send message: reliable stream is not ready yet");
        return;
    };

    let payload = match codec::encode(&message) {
        Ok(payload) => payload,
        Err(error) => {
            tracing::error!("failed to encode ClientGameMessage: {error:#}");
            return;
        }
    };

    if let Err(error) = peer.send(&connection, &stream, Bytes::from(payload)) {
        tracing::error!("failed to send ClientGameMessage: {}", error);
    }
}
