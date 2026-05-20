use crate::net::gameplay_quic::{
    send_message, GameplayClient
};
use crate::world::state::LocalWorldState;
use bevy::prelude::*;
use bevy::app::AppExit;
use shared::protocol::ClientGameMessage;

pub fn keyboard_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut gameplay_client: ResMut<GameplayClient>,
    mut app_exit: MessageWriter<AppExit>,
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
        app_exit.write(AppExit::Success);
    }

    if !keyboard.just_pressed(KeyCode::Escape) && keyboard.get_just_pressed().next().is_some() {
        send_message(&mut gameplay_client, ClientGameMessage::Heartbeat);
    }
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

pub fn handle_input_accepted(
    world_state: &mut LocalWorldState,
    movement_x: f32,
    movement_y: f32,
) {
    world_state.last_movement_x = movement_x;
    world_state.last_movement_y = movement_y;

    tracing::info!("input accepted: x={} y={}", movement_x, movement_y);
}