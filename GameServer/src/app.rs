use crate::config::ServerConfig;
use crate::net::heartbeat::{bind_heartbeat_socket, send_heartbeat};
use crate::net::network_event::{
    SharedPlayerRegistry, connect_to_broker, poll_broker_events,
    publish_world_snapshots, publish_gameplay_updates,
};
use crate::world::combat::{
    ColorSwapTimer, PendingActions, PendingSwapEvents, PlayerCombatRegistry,
    color_swap_system, player_combat_system, score_collection_system,
};
use crate::world::enemy::{EnemyRegistry, enemy_spawn_system, enemy_ai_system};
use crate::world::projectile::{
    ProjectileRegistry, projectile_movement_system, projectile_collision_system,
};
use crate::world::state::{PlayerRegistry, update_players_registry, sync_combat_registry};
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
            tracing::error!("failed to start GameServer: {error:#}");
            return;
        }
    };

    App::new()
        .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            Duration::from_millis(1000 / DEFAULT_DS_TICK_RATE),
        )))
        .insert_resource(config)
        .insert_resource(SharedPlayerRegistry {
            inner: Arc::new(Mutex::new(PlayerRegistry::default())),
        })
        // ── 5SecsSwap resources ────────────────────────────────────────────
        .insert_resource(EnemyRegistry::default())
        .insert_resource(ProjectileRegistry::default())
        .insert_resource(ColorSwapTimer::default())
        .insert_resource(PendingActions::default())
        .insert_resource(PendingSwapEvents::default())
        .insert_resource(PlayerCombatRegistry::default())
        // ── Startup ────────────────────────────────────────────────────────
        .add_systems(Startup, (bind_heartbeat_socket, connect_to_broker))
        // ── Update ─────────────────────────────────────────────────────────
        .add_systems(
            Update,
            (
                // Network ingress.
                poll_broker_events,
                sync_combat_registry,
                // Physics & AI.
                update_players_registry,
                enemy_spawn_system,
                enemy_ai_system,
                projectile_movement_system,
                // Combat.
                color_swap_system,
                player_combat_system,
                projectile_collision_system,
                score_collection_system,
                // Network egress.
                publish_world_snapshots,
                publish_gameplay_updates,
                send_heartbeat,
            )
                .chain(),
        )
        .run();
}