use crate::net::gameplay_quic::{
    send_message, send_player_input, GameplayClient,
};
use bevy::prelude::*;
use shared::protocol::ClientGameMessage;

pub fn keyboard_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut gameplay_client: ResMut<GameplayClient>,
) {
    if keyboard.just_pressed(KeyCode::KeyW) {
        send_player_input(&mut gameplay_client, 0.0, 1.0);
    }

    if keyboard.just_pressed(KeyCode::KeyS) {
        send_player_input(&mut gameplay_client, 0.0, -1.0);
    }

    if keyboard.just_pressed(KeyCode::KeyA) {
        send_player_input(&mut gameplay_client, -1.0, 0.0);
    }

    if keyboard.just_pressed(KeyCode::KeyD) {
        send_player_input(&mut gameplay_client, 1.0, 0.0);
    }

    if keyboard.just_pressed(KeyCode::Space) {
        send_player_input(&mut gameplay_client, 0.0, 0.0);
    }

    if keyboard.just_pressed(KeyCode::Escape) {
        send_message(&mut gameplay_client, ClientGameMessage::LeaveGame);
    }

    if !keyboard.just_pressed(KeyCode::Escape) && keyboard.get_just_pressed().next().is_some() {
        send_message(&mut gameplay_client, ClientGameMessage::Heartbeat);
    }
}