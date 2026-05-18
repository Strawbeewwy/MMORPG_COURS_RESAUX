use crate::config::ClientConfig;
use crate::state::LocalPlayerState;
use bevy::prelude::*;
use bytes::Bytes;
use shared::protocol::transport::codec;
use shared::protocol::transport::game_sockets_quic::QuicBackend;
use shared::protocol::transport::gamesockets_lib::{
    GameConnection, GameNetworkEvent, GamePeer, GameStream, GameStreamReliability,
};
use shared::protocol::{ClientGameMessage, ServerGameMessage};
use shared::config::GAME_PROTOCOL_VERSION;

#[derive(Resource, Default)]
pub struct GameplayClient {
    pub peer: Option<GamePeer>,
    pub connection: Option<GameConnection>,
    pub reliable_stream: Option<GameStream>,
    pub joined: bool,
}

pub fn connect_to_game_server(
    config: Res<ClientConfig>,
    mut gameplay_client: ResMut<GameplayClient>,
    mut player_state: ResMut<LocalPlayerState>,
) {
    tracing::info!(
        "starting GameClient for username={} player_id={} server={} zone={}",
        config.username,
        config.player_id,
        config.server_addr(),
        config.zone
    );

    player_state.player_id = Some(config.player_id.clone());
    player_state.zone = Some(config.zone.clone());

    let peer = GamePeer::new(QuicBackend::new());

    if let Err(error) = peer.connect(&config.server_ip, config.server_port) {
        tracing::error!(
            "failed to connect to game server {}:{}: {}",
            config.server_ip,
            config.server_port,
            error
        );
        return;
    }

    gameplay_client.peer = Some(peer);

    tracing::info!("connecting to game server {}...", config.server_addr());
}

pub fn poll_gameplay_events(
    config: Res<ClientConfig>,
    mut gameplay_client: ResMut<GameplayClient>,
    mut player_state: ResMut<LocalPlayerState>,
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
                    tracing::error!("failed to poll game peer: {}", error);
                    break;
                }
            }
        };

        handle_gameplay_event(&config, &mut gameplay_client, &mut player_state, event);
    }
}

fn handle_gameplay_event(
    config: &ClientConfig,
    gameplay_client: &mut GameplayClient,
    player_state: &mut LocalPlayerState,
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

        GameNetworkEvent::StreamClosed(connection, stream) => {
            tracing::info!(
                "stream closed: connection={} stream={}",
                connection.connection_id,
                stream.stream_id
            );

            gameplay_client.reliable_stream = None;
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

            handle_server_message(gameplay_client, player_state, &data);
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

fn handle_server_message(
    gameplay_client: &mut GameplayClient,
    player_state: &mut LocalPlayerState,
    data: &[u8],
) {
    let message = match codec::decode::<ServerGameMessage>(data) {
        Ok(message) => message,
        Err(error) => {
            tracing::warn!("failed to decode ServerGameMessage: {error:#}");
            return;
        }
    };

    match message {
        ServerGameMessage::JoinAccepted {
            player_id,
            player,
            snapshot,
            message,
        } => {
            gameplay_client.joined = true;
            player_state.player_id = Some(player_id.clone());
            player_state.zone = Some(snapshot.zone.clone());

            tracing::info!(
                "join accepted: player_id={} username={} message={} zone={} players={}",
                player_id,
                player.username,
                message,
                snapshot.zone,
                snapshot.players.len()
            );
        }

        ServerGameMessage::JoinRejected { reason } => {
            tracing::warn!("join rejected: {}", reason);
        }

        ServerGameMessage::HeartbeatAck => {
            tracing::info!("heartbeat ack");
        }

        ServerGameMessage::InputAccepted {
            movement_x,
            movement_y,
        } => {
            player_state.last_movement_x = movement_x;
            player_state.last_movement_y = movement_y;

            tracing::info!("input accepted: x={} y={}", movement_x, movement_y);
        }

        ServerGameMessage::WorldSnapshot { snapshot } => {
            player_state.zone = Some(snapshot.zone.clone());

            tracing::info!(
                "world snapshot: zone={} players={}",
                snapshot.zone,
                snapshot.players.len()
            );
        }

        ServerGameMessage::PlayerJoined { player } => {
            tracing::info!(
                "player joined: id={} username={}",
                player.player_id,
                player.username
            );
        }

        ServerGameMessage::PlayerLeft { player_id } => {
            tracing::info!("player left: {}", player_id);
        }

        ServerGameMessage::Goodbye => {
            tracing::info!("server said goodbye");
            gameplay_client.joined = false;
        }
    }
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
            protocol_version: GAME_PROTOCOL_VERSION,
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
