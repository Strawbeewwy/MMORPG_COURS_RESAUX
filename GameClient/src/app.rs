use crate::config::ClientConfig;
use crate::net::broker_client::BrokerClient;
use crate::net::broker_connection::{
    connect_to_broker, poll_broker_events, retry_broker_connection_if_needed,
};
use crate::net::input::keyboard_input_system;
use crate::render::entity_renderer::render_entities;
use crate::world::state::LocalWorldState;
use bevy::prelude::*;

pub fn run() {
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
        .insert_resource(LocalWorldState::default())
        .insert_resource(BrokerClient::default())
        .add_systems(Startup, (setup_camera, connect_to_broker))
        .add_systems(
            Update,
            (
                poll_broker_events,
                retry_broker_connection_if_needed,
                keyboard_input_system,
                render_entities,
            ),
        )
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}