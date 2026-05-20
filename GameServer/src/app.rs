

use crate::config::ServerConfig;
use crate::net::network_event::{
    poll_network_events,start_quic_server,
    SharedPlayerRegistry, broadcast_world_snapshots,
};
use crate::net::heartbeat::{
    bind_heartbeat_socket, send_heartbeat
};
use crate::world::state::{
    PlayerRegistry, update_players_registry
};
use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use shared::config::DEFAULT_DS_TICK_RATE;

pub fn run() {
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    App::new()
        .add_plugins(MinimalPlugins
            .set(ScheduleRunnerPlugin::run_loop({
                Duration::from_millis(1000/DEFAULT_DS_TICK_RATE)
            })))
        .insert_resource(ServerConfig::from_env())
        .insert_resource(SharedPlayerRegistry {
            inner: Arc::new(Mutex::new(PlayerRegistry::default())),
        })
        .add_systems(
            Startup,
            (
            bind_heartbeat_socket,
            start_quic_server)
            )
        .add_systems(
            Update,
            (
                poll_network_events,//get player input
                update_players_registry,//update player registry
                broadcast_world_snapshots,//send updated world state
                send_heartbeat,
            )
                .chain(),
        )
        .run();
}