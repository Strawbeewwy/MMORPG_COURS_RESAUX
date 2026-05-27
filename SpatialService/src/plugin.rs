use bevy::prelude::*;
use crate::messages::{CrossingAlertMsg, PositionUpdateMsg};
use crate::net::broker_client::connect_to_broker;
use crate::net::shard_listener::bind_shard_listener;
use crate::resources::client_map::ClientMap;
use crate::systems::crossing::handle_crossing_alerts;
use crate::systems::receive::{poll_broker_connection, poll_shard_events};
use crate::systems::subscriptions::handle_subscriptions;

pub struct SpatialPlugin;

impl Plugin for SpatialPlugin {
    fn build(&self, app: &mut App) {
        app
            // Bevy messages — cleared automatically each frame
            .add_message::<PositionUpdateMsg>()
            .add_message::<CrossingAlertMsg>()
            // Resources
            .init_resource::<ClientMap>()
            // Startup: open sockets
            .add_systems(Startup, (bind_shard_listener, connect_to_broker))
            // Update: poll → dispatch → react (chained to guarantee ordering)
            .add_systems(
                Update,
                (
                    poll_shard_events,       // fill PositionUpdate event queue
                    poll_broker_connection,  // advance broker handshake
                    handle_subscriptions,    // process positions, send Subscribe/Unsubscribe
                    handle_crossing_alerts,  // log CrossingAlert, stub for Part 3
                )
                    .chain(),
            );
    }
}

