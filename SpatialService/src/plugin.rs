use bevy::prelude::*;
use crate::messages::{CrossingAlertMsg, HandoffRequestMsg, PositionUpdateMsg};
use crate::net::broker_client::{connect_to_broker, reconnect_broker_if_needed};
use crate::net::shard_listener::bind_shard_listener;
use crate::resources::client_map::ClientMap;
use crate::resources::crossing_cooldowns::CrossingCooldowns;
use crate::systems::crossing::handle_crossing_alerts;
use crate::systems::handoff::handle_handoff_requests;
use crate::systems::receive::{poll_broker_connection, poll_shard_events};
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
            .init_resource::<CrossingCooldowns>()
            // Startup: open sockets
            .add_systems(Startup, (bind_shard_listener, connect_to_broker))
            // Update: poll → reconnect → dispatch → react → handoff (chained for ordering)
            .add_systems(
                Update,
                (
                    poll_shard_events,           // decode PositionUpdate/ShardRegister/HandoffAck, clean ClientMap on disconnect
                    poll_broker_connection,       // advance broker handshake state
                    reconnect_broker_if_needed,   // retry on Disconnected state
                    handle_subscriptions,         // Subscribe/Unsubscribe + emit CrossingAlertMsg
                    handle_crossing_alerts,       // CrossingAlertMsg → HandoffRequestMsg
                    handle_handoff_requests,      // HandoffRequestMsg → wire HandoffRequest to destination shard
                )
                    .chain(),
            );
    }
}
