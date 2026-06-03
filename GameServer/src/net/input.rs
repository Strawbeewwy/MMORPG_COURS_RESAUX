use crate::config::ServerConfig;
use crate::net::network_event::SharedPlayerRegistry;
use shared::protocol::{ClientId, CLIENT_INPUT_LEN};
use shared::protocol::NetVec2;

pub fn handle_broker_client_input(
    config: &ServerConfig,
    registry: &SharedPlayerRegistry,
    client_id: ClientId,
    input: [u8; CLIENT_INPUT_LEN],
) {

    let movement_x = read_f32_le(&input[0..4]);
    let movement_y = read_f32_le(&input[4..8]);


    if !movement_x.is_finite() || !movement_y.is_finite() {
        tracing::warn!(
           "invalid client input: client_id={} movement_x={} movement_y={}",
           client_id.0,
           movement_x,
           movement_y
       );
        return;
    }

    let Ok(mut registry) = registry.inner.try_lock() else {
        tracing::warn!("could not lock player registry for client input");
        return;
    };

    let Some(&player_id) = registry.client_player.get(&client_id) else {
        tracing::warn!("player not found for client input with id: {}", client_id.0);
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