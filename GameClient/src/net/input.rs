use crate::net::broker_client::BrokerClient;
use crate::world::state::LocalWorldState;
use bevy::app::AppExit;
use bevy::prelude::*;
use shared::protocol::{
    CLIENT_INPUT_LEN, encode_message, NetworkMessage,
    ClientId,
};



pub fn keyboard_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut broker_client: ResMut<BrokerClient>,
    mut app_exit: MessageWriter<AppExit>,
    mut world_state: ResMut<LocalWorldState>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        app_exit.write(AppExit::Success);
        return;
    }
    
    let Some(client_id) = broker_client.client_id else {
        return;
    };

    let movement_x = get_input_x_axis(&keyboard);
    let movement_y = get_input_y_axis(&keyboard);

    if movement_x != world_state.last_movement_x || movement_y != world_state.last_movement_y {
        send_player_input(
            &mut broker_client,
            client_id,
            movement_x,
            movement_y,
        );

        world_state.last_movement_x = movement_x;
        world_state.last_movement_y = movement_y;
    }
}

pub fn send_player_input(
    broker_client: &mut BrokerClient,
    client_id: ClientId,
    movement_x: f32,
    movement_y: f32,
) {
    let input = encode_movement_input(movement_x, movement_y);

    let packet = match encode_message(&NetworkMessage::ClientInput {
        client_id,
        input,
    }) {
        Ok(packet) => packet,
        Err(error) => {
            tracing::warn!(
                "cannot encode ClientInput for client {}: {}",
                client_id.0,
                error
            );
            return;
        }
    };
    broker_client.send_raw(packet);
}

fn encode_movement_input(
    movement_x: f32,
    movement_y: f32,
) -> [u8; CLIENT_INPUT_LEN] {
    let mut input = [0_u8; CLIENT_INPUT_LEN];

    input[0..4].copy_from_slice(&movement_x.to_le_bytes());
    input[4..8].copy_from_slice(&movement_y.to_le_bytes());

    input
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

// Can later add more input types, like gamepads
pub fn get_input_x_axis(
    keyboard: &Res<ButtonInput<KeyCode>>,
) -> f32 {
    (keyboard.pressed(KeyCode::KeyD) as i8 - keyboard.pressed(KeyCode::KeyA) as i8) as f32
}

// Can later add more input types, like gamepads
pub fn get_input_y_axis(
    keyboard: &Res<ButtonInput<KeyCode>>,
) -> f32 {
    (keyboard.pressed(KeyCode::KeyW) as i8 - keyboard.pressed(KeyCode::KeyS) as i8) as f32
}