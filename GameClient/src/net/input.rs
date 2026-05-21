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
    mut world_state: ResMut<LocalWorldState>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        send_message(&mut gameplay_client, ClientGameMessage::LeaveGame);
        app_exit.write(AppExit::Success);
        return;
    }

    let movement_x = get_input_x_axis(&keyboard);
    let movement_y = get_input_y_axis(&keyboard);

    if movement_x != world_state.last_movement_x || movement_y != world_state.last_movement_y {
        send_player_input(&mut gameplay_client, movement_x, movement_y);
        world_state.last_movement_x = movement_x;
        world_state.last_movement_y = movement_y;
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

//Can later add more input types, like gamepads
pub fn get_input_x_axis(
    keyboard: &Res<ButtonInput<KeyCode>>,
) -> f32{
    (keyboard.pressed(KeyCode::KeyD) as i8 - keyboard.pressed(KeyCode::KeyA) as i8) as f32
}

//Can later add more input types, like gamepads
pub fn get_input_y_axis(
    keyboard: &Res<ButtonInput<KeyCode>>,
) -> f32{
    (keyboard.pressed(KeyCode::KeyW) as i8 - keyboard.pressed(KeyCode::KeyS) as i8) as f32
}

