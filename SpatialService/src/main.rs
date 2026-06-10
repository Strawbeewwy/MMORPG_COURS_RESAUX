mod messages;
mod net;
mod plugin;
mod resources;
mod systems;

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use std::time::Duration;

use plugin::SpatialPlugin;
use resources::config::SpatialConfig;
use resources::quad_tree::{QuadTree, Rect};
use shared::config::DEFAULT_DS_TICK_RATE;

fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    let config = SpatialConfig::from_env();

    // Build the QuadTree from config before moving config into Bevy
    let world_bounds = Rect::world(config.world_half_size);
    let quad_tree = QuadTree::new(world_bounds, config.quad_tree_max_depth);

    tracing::info!(
        "spatial service starting — listen={}:{} utils={}:{} depth={} margin={}",
        config.listen_host,
        config.listen_port,
        config.broker_host,
        config.broker_port,
        config.quad_tree_max_depth,
        config.crossing_margin,
    );

    App::new()
        .add_plugins(
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(
                1000 / DEFAULT_DS_TICK_RATE,
            ))),
        )
        .insert_resource(config)
        .insert_resource(quad_tree)
        .add_plugins(SpatialPlugin)
        .run();
}

