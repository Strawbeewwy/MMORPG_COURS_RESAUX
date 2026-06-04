use crate::config::ServerConfig;
use crate::net::network_event::SharedPlayerRegistry;
use crate::world::combat::PendingActions;
use shared::protocol::broker::{ClientId, CLIENT_INPUT_LEN};
use shared::protocol::NetVec2;
use bevy::prelude::*;

pub fn handle_broker_client_input(
    _config: &ServerConfig,
    registry: &SharedPlayerRegistry,
    pending: &mut PendingActions,
    client_id: ClientId,
    input: [u8; CLIENT_INPUT_LEN],
) {
    let movement_x = read_f32_le(&input[0..4]);
    let movement_y = read_f32_le(&input[4..8]);

    if !movement_x.is_finite() || !movement_y.is_finite() {
        tracing::warn!(
            "invalid client input: client_id={} movement_x={} movement_y={}",
            client_id.0, movement_x, movement_y
        );
        return;
    }

    // Byte 8: action flags bitmask (dash=bit0, melee=bit1, shoot=bit2).
    let action_flags = input[8];
    // Bytes 9..13 = look_x, bytes 13..17 = look_y.
    let look_x = read_f32_le(&input[9..13]);
    let look_y = read_f32_le(&input[13..17]);
    let look_dir = bevy::prelude::Vec2::new(
        if look_x.is_finite() { look_x } else { movement_x },
        if look_y.is_finite() { look_y } else { movement_y },
    );

    // Store actions for combat processing.
    pending.actions.insert(client_id, (action_flags, look_dir));

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
        "client input applied: client_id={} dx={} dy={} flags=0b{:08b}",
        player_id, movement_x, movement_y, action_flags
    );
}

fn read_f32_le(bytes: &[u8]) -> f32 {
    if bytes.len() < 4 { return 0.0; }
    f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}