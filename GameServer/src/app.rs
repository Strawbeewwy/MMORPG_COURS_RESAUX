use crate::config::ServerConfig;

use crate::net::{
    send_heartbeat, bind_heartbeat_socket,
    connect_to_broker,
    poll_broker_events,
    publish_world_update, publish_player_position_updates
};
use crate::world::{ClientEntityRegistry, EntityRegistry, SpawnGenericEntityEvent, SpawnGhostEntityEvent, SpawnPlayerEntityEvent, SharedEntityRegistry, EntityIdAllocator};

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use shared::config::DEFAULT_DS_TICK_RATE;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use crate::net::network_event::reconnect_broker_if_needed;
use crate::world::spawn_entity::{spawn_generic_entities, spawn_ghost_entities, spawn_player_entities};

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
        .insert_resource(SharedEntityRegistry {
            entity_reg_shared: Arc::new(Mutex::new(EntityRegistry::default())),
            client_reg_shared: Arc::new(Mutex::new(ClientEntityRegistry::default()))
        })
        .insert_resource(EntityIdAllocator::default())
        .add_message::<SpawnPlayerEntityEvent>()
        .add_message::<SpawnGhostEntityEvent>()
        .add_message::<SpawnGenericEntityEvent>()
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
                reconnect_broker_if_needed,
                spawn_player_entities,
                spawn_ghost_entities,
                spawn_generic_entities,
                publish_player_position_updates,
                publish_world_update,
                send_heartbeat,
            )
                .chain(),
        )
        .run();
}