use crate::config::ServerConfig;
use crate::net::network_event::SharedPlayerRegistry;
use crate::world::player::{PLAYER_DEFAULT_MOVE_SPEED, PlayerInfo};
use shared::protocol::broker::{ClientId, CLIENT_INPUT_LEN};
use shared::protocol::NetVec2;

pub fn handle_broker_client_input(
    config: &ServerConfig,
    registry: &SharedPlayerRegistry,
    client_id: ClientId,
    input: [u8; CLIENT_INPUT_LEN],
) {
    let movement_x = read_f32_le(&input[0..4]);
    let movement_y = read_f32_le(&input[4..8]);

    let player_id = client_id.to_string();

    let Ok(mut registry) = registry.inner.try_lock() else {
        tracing::warn!("could not lock player registry for client input");
        return;
    };

    let player = registry
        .players
        .entry(player_id.clone())
        .or_insert_with(|| {
            tracing::info!(
                "creating shard player from broker client_id={} zone={}",
                client_id,
                config.zone
            );

            PlayerInfo {
                player_id: player_id.clone(),
                username: format!("player_{client_id}"),
                zone: config.zone.clone(),
                position: NetVec2::ZERO,
                velocity: NetVec2::ZERO,
                movement_speed: PLAYER_DEFAULT_MOVE_SPEED,
            }
        });

    player.velocity = NetVec2::new(movement_x, movement_y);

    tracing::debug!(
        "client input applied: client_id={} movement_x={} movement_y={}",
        client_id,
        movement_x,
        movement_y
    );
}

fn read_f32_le(bytes: &[u8]) -> f32 {
    f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}