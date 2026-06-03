use crate::config::ServerConfig;
use crate::net::heartbeat::{
    bind_heartbeat_socket, send_heartbeat
};
use crate::net::network_event::{
    SharedPlayerRegistry, connect_to_broker,
    poll_broker_events, publish_world_update,
    publish_player_position_updates};
use crate::world::state::{
    EntityRegistry, update_players_registry
};
use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use shared::config::DEFAULT_DS_TICK_RATE;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

pub fn run() {
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();


    let config = match ServerConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            tracing::error!("failed to start GameClient: {error:#}");
            return;
        }
    };

    App::new()
        .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            Duration::from_millis(1000 / DEFAULT_DS_TICK_RATE),
        )))
        .insert_resource(config)
        .insert_resource(SharedPlayerRegistry {
            inner: Arc::new(Mutex::new(EntityRegistry::default())),
        })
        .add_systems(
            Startup,
            (
                bind_heartbeat_socket,
                connect_to_broker,
            ),
        )
        .add_systems(
            Update,
            (
                poll_broker_events,
                update_players_registry,
                publish_player_position_updates,
                publish_world_update,
                send_heartbeat,
            )
                .chain(),
        )
        .run();
}