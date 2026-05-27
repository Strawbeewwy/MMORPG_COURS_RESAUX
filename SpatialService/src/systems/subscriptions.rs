use bevy::prelude::*;
use shared::protocol::broker::{Subscribe, Unsubscribe};
use crate::messages::{CrossingAlertMsg, PositionUpdateMsg};
use crate::resources::client_map::ClientMap;
use crate::resources::config::SpatialConfig;
use crate::resources::net_handles::BrokerClient;
use crate::resources::quad_tree::QuadTree;

/// Consume PositionUpdateMsg messages, resolve the shard via the QuadTree,
/// and send Subscribe / Unsubscribe to the broker when the shard changes.
/// Also emits CrossingAlertMsg messages when the client is near a boundary.
pub fn handle_subscriptions(
    mut ev_positions: MessageReader<PositionUpdateMsg>,
    mut ev_crossings: MessageWriter<CrossingAlertMsg>,
    quad_tree: Res<QuadTree>,
    mut client_map: ResMut<ClientMap>,
    broker: Res<BrokerClient>,
    config: Res<SpatialConfig>,
) {
    for update in ev_positions.read() {
        let new_shard = quad_tree.shard_for(update.x, update.y);
        let old_shard = client_map.0.get(&update.client_id).copied();

        // Unsubscribe from the previous shard, subscribe to the new one
        if new_shard != old_shard {
            if let Some(old) = old_shard {
                broker.send(Unsubscribe::new(update.client_id, old).to_bytes());
                tracing::debug!("client {} unsubscribed from shard:{old}", update.client_id);
            }

            if let Some(new) = new_shard {
                broker.send(Subscribe::new(update.client_id, new).to_bytes());
                client_map.0.insert(update.client_id, new);
                tracing::debug!("client {} subscribed to shard:{new}", update.client_id);
            }
        }

        // Detect proximity to a shard boundary
        let nearby = quad_tree.shards_near(update.x, update.y, config.crossing_margin);
        if nearby.len() > 1 {
            ev_crossings.write(CrossingAlertMsg {
                client_id: update.client_id,
                shards: nearby,
            });
        }
    }
}

