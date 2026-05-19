use crate::config::ServerConfig;
use crate::net::network_event::{GameplayPeer, SharedPlayerRegistry};
use crate::world::player::PlayerInfo;
use shared::config::GAME_PROTOCOL_VERSION;
use shared::game_sockets::GameConnection;
use shared::protocol::{NetVec2, ServerGameMessage};
use uuid::Uuid;

pub fn handle_join_game(
    config: &ServerConfig,
    gameplay_peer: &mut GameplayPeer,
    registry: &SharedPlayerRegistry,
    connection: GameConnection,
    protocol_version: String,
    session_token: String,
    username: String,
) -> ServerGameMessage {
    if GAME_PROTOCOL_VERSION != protocol_version {
        return ServerGameMessage::JoinRejected {
            reason: format!(
                "unsupported_protocol_version: expected {}, got {}",
                GAME_PROTOCOL_VERSION, protocol_version
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

    if let Some(player_id) = gameplay_peer.connection_players.get(&connection) {
        if let Some(existing_player) = registry.players.get(player_id) {
            let snapshot = registry.snapshot(config.zone.clone());

            return ServerGameMessage::JoinAccepted {
                player_id: existing_player.player_id.clone(),
                player: existing_player.public_info(),
                snapshot,
                message: "already_joined".to_string(),
            };
        }

        gameplay_peer.connection_players.remove(&connection);
    }

    if registry.is_full(config.max_players) {
        return ServerGameMessage::JoinRejected {
            reason: "server_full".to_string(),
        };
    }

    let player_id = Uuid::new_v4().to_string();

    let player = PlayerInfo {
        player_id: player_id.clone(),
        username: username.trim().to_string(),
        zone: config.zone.clone(),
        position: NetVec2::ZERO,
        velocity: NetVec2::ZERO,
    };

    let public_player = player.public_info();

    registry.players.insert(player_id.clone(), player);
    gameplay_peer
        .connection_players
        .insert(connection, player_id.clone());

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

pub fn handle_leave_game(
    gameplay_peer: &mut GameplayPeer,
    registry: &SharedPlayerRegistry,
    connection: GameConnection,
) -> ServerGameMessage {
    remove_player_for_connection(gameplay_peer, registry, connection);

    ServerGameMessage::Goodbye
}

pub fn remove_player_for_connection(
    gameplay_peer: &mut GameplayPeer,
    registry: &SharedPlayerRegistry,
    connection: GameConnection,
) {
    if let Some(player_id) = gameplay_peer.connection_players.remove(&connection) {
        if let Ok(mut registry) = registry.inner.try_lock() {
            if let Some(player) = registry.players.remove(&player_id) {
                tracing::info!(
                    "player left: player_id={} connection={}",
                    player.player_id,
                    connection.connection_id
                );
            }
        }
    }
}