

use crate::config::ServerConfig;
use crate::net::gameplay_quic::{
    poll_gameplay_events, start_gameplay_quic_server, SharedPlayerRegistry,
};
use crate::net::heartbeat::{bind_heartbeat_socket, send_heartbeat};
use crate::world::state::PlayerRegistry;
use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

pub fn run() {
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    App::new()
        .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(16))))
        .insert_resource(ServerConfig::from_env())
        .insert_resource(SharedPlayerRegistry {
            inner: Arc::new(Mutex::new(PlayerRegistry::default())),
        })
        .add_systems(Startup, (bind_heartbeat_socket, start_gameplay_quic_server))
        .add_systems(Update, (poll_gameplay_events, send_heartbeat))
        .run();
}