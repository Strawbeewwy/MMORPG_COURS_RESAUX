use crate::net::broker_client::BrokerClient;
use crate::world::state::LocalWorldState;
use shared::protocol::{PlayerPublicInfo, WorldSnapshot};

pub fn handle_join_accepted(
    broker_client: &mut BrokerClient,
    world_state: &mut LocalWorldState,
    player_id: String,
    player: PlayerPublicInfo,
    snapshot: WorldSnapshot,
    message: String,
) {
    broker_client.connected = true;
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

pub fn handle_goodbye(broker_client: &mut BrokerClient) {
    tracing::info!("server said goodbye");
    broker_client.connected = false;
}