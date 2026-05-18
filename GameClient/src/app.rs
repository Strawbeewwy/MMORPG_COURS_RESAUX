use crate::config::ClientConfig;
use crate::state::LocalPlayerState;
use crate::input::keyboard_input_system;
use crate::net::gameplay_quic::{
    connect_to_game_server, poll_gameplay_events,
};
use bevy::prelude::*;

pub fn run() {
    tracing_subscriber::fmt::init();


    let config = match ClientConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            tracing::error!("failed to start GameClient: {error:#}");
            return;
        }
    };

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "MMORPG Lab - Bevy Game Client".to_string(),
                resolution: (900, 600).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(config)
        .insert_resource(LocalPlayerState::default())
        .add_systems(Startup, (setup_camera, connect_to_game_server))
        .add_systems(
            Update,
            (
                poll_gameplay_events,
                keyboard_input_system,
            ),
        )
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}