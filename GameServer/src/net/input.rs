use crate::config::ServerConfig;
use crate::net::network_event::SharedPlayerRegistry;
use shared::protocol::broker::CLIENT_INPUT_LEN;
use shared::protocol::NetVec2;
use shared::protocol::game::PlayerId;

pub fn handle_broker_client_input(
    config: &ServerConfig,
    registry: &SharedPlayerRegistry,
    player_id: PlayerId,
    input: [u8; CLIENT_INPUT_LEN],
) {
    let movement_x = read_f32_le(&input[0..4]);
    let movement_y = read_f32_le(&input[4..8]);


    let Ok(mut registry) = registry.inner.try_lock() else {
        tracing::warn!("could not lock player registry for client input");
        return;
    };

    let Some(player) = registry.players.get_mut(&player_id) else {
        tracing::warn!("player not found for client input with id: {}", player_id);
        return;
    };

    player.velocity = NetVec2::from_f32(movement_x, movement_y, NetVec2::DEFAULT_PRECISION);

    tracing::debug!(
        "client input applied: client_id={} movement_x={} movement_y={}",
        player_id,
        movement_x,
        movement_y
    );
}

fn read_f32_le(bytes: &[u8]) -> f32 {
    f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}