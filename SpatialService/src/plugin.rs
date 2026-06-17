use bevy::prelude::*;
use crate::messages::{CrossingAlertMsg, HandoffRequestMsg, PositionUpdateMsg};
use crate::net::broker_client::{connect_to_broker, reconnect_broker_if_needed};
use crate::net::orchestrator_client::connect_to_orchestrator;
use crate::resources::client_map::ClientMap;
use crate::resources::crossing_cooldowns::CrossingCooldowns;
use crate::resources::entity_map::EntityMap;
use crate::resources::handoff_queue::PendingHandoffs;
use crate::systems::crossing::handle_crossing_alerts;
use crate::systems::handoff::handle_handoff_start;
use crate::systems::receive::{poll_broker_connection};
use crate::systems::subscriptions::handle_subscriptions;

pub struct SpatialPlugin;

impl Plugin for SpatialPlugin {
    fn build(&self, app: &mut App) {
        app
            // Bevy messages — cleared automatically each frame
            .add_message::<PositionUpdateMsg>()
            .add_message::<CrossingAlertMsg>()
            .add_message::<HandoffRequestMsg>()
            // Resources
            .init_resource::<ClientMap>()
            .init_resource::<EntityMap>()
            .init_resource::<CrossingCooldowns>()
            .init_resource::<PendingHandoffs>()
            // Startup: open sockets
            .add_systems(Startup, (connect_to_broker, connect_to_orchestrator))
            // Update: poll → reconnect → dispatch → react → handoff (chained for ordering)
            .add_systems(
                Update,
                (
                    poll_broker_connection,       // advance utils handshake state
                    reconnect_broker_if_needed,   // retry on Disconnected state
                    handle_subscriptions,         // update positions, split overloaded shards, emit handoffs
                    handle_crossing_alerts,       // CrossingAlertMsg → HandoffRequestMsg
                    handle_handoff_start,      // HandoffRequestMsg → wire HandoffRequest to destination shard
                )
                    .chain(),
            );
    }
}
