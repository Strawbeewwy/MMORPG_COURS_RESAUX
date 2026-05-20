use crate::config::ClientConfig;
use crate::net::gameplay_quic::{send_message, GameplayClient};
use crate::world::state::LocalWorldState;
use shared::config::GAME_PROTOCOL_VERSION;
use shared::protocol::{PlayerPublicInfo, WorldSnapshot};

pub fn send_join_game(config: &ClientConfig, gameplay_client: &mut GameplayClient) {
    tracing::info!(
        "sending JoinGame username={} session_token={}",
        config.username,
        config.player_id
    );

    send_message(
        gameplay_client,
        shared::protocol::ClientGameMessage::JoinGame {
            protocol_version: GAME_PROTOCOL_VERSION.to_string(),
            session_token: config.player_id.clone(),
            username: config.username.clone(),
        },
    );
}

pub fn handle_join_accepted(
    gameplay_client: &mut GameplayClient,
    world_state: &mut LocalWorldState,
    player_id: String,
    player: PlayerPublicInfo,
    snapshot: WorldSnapshot,
    message: String,
) {
    gameplay_client.joined = true;
    world_state.player_id = Some(player_id.clone());
    world_state.zone = Some(snapshot.zone.clone());
    world_state.set_players_from_snapshot(snapshot.players.clone());
    world_state.rebuild_render_entities();

    tracing::info!(
        "join accepted: player_id={} username={} message={} zone={} players={}",
        player_id,
        player.username,
        message,
        snapshot.zone,
        snapshot.players.len()
    );

}

pub fn handle_join_rejected(reason: String) {
    tracing::warn!("join rejected: {}", reason);
}

pub fn handle_goodbye(gameplay_client: &mut GameplayClient) {
    tracing::info!("server said goodbye");
    gameplay_client.joined = false;
}