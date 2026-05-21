use crate::config::BrokerConfig;
use crate::net::network_event::{poll_broker_events, start_broker};
use crate::pubsub::state::PubSubState;
use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use std::time::Duration;

const BROKER_TICK_RATE: u64 = 60;

pub fn run() {
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    App::new()
        .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            Duration::from_millis(1000 / BROKER_TICK_RATE),
        )))
        .insert_resource(BrokerConfig::from_env())
        .init_resource::<PubSubState>()
        .add_systems(Startup, start_broker)
        .add_systems(Update, poll_broker_events)
        .run();
}