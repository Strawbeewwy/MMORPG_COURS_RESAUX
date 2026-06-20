use bevy::prelude::*;
use bevy::platform::collections::HashSet;
use shared::protocol::{encode_message, NetworkMessage, ShardId, Topic};
use crate::resources::config::SpatialConfig;
use crate::resources::entity_map::EntityMap;
use crate::resources::net_handles::BrokerClient;
use crate::resources::quad_tree::QuadTree;

/// Manage AOI-based subscriptions: subscribe entities to all shards within their AOI radius.
/// This allows entities near shard boundaries to receive updates from multiple shards.
pub fn manage_aoi_subscriptions(
    mut entity_map: ResMut<EntityMap>,
    quad_tree: Res<QuadTree>,
    config: Res<SpatialConfig>,
    broker: Res<BrokerClient>,
) {
    if !broker.handle.is_ready() {
        return;
    }

    let mut nearby_buf = Vec::with_capacity(4);

    for record in entity_map.entities.values_mut() {
        if !record.is_player() {
            continue;
        }

        nearby_buf.clear();
        quad_tree.shards_near_into(
            record.position.x,
            record.position.y,
            config.aoi_radius,
            &mut nearby_buf,
        );

        let new_shards: HashSet<ShardId> = nearby_buf.iter().copied().collect();

        let to_subscribe: Vec<ShardId> = new_shards
            .difference(&record.subscribed_shards)
            .copied()
            .collect();

        let to_unsubscribe: Vec<ShardId> = record
            .subscribed_shards
            .difference(&new_shards)
            .copied()
            .collect();

        for shard_id in to_subscribe {
            let message = NetworkMessage::Subscribe {
                client_id: record.client_id,
                topic: Topic::ShardInstance { id: shard_id },
            };

            if let Ok(packet) = encode_message(&message) {
                if let Err(e) = broker.handle.send(packet) {
                    tracing::error!(
                        "AOI: failed to subscribe client {} to shard {}: {e:#}",
                        record.client_id.0,
                        shard_id.0
                    );
                    continue;
                }

                record.subscribed_shards.insert(shard_id);

                tracing::debug!(
                    "AOI: subscribed client {} to shard {}",
                    record.client_id.0,
                    shard_id.0
                );
            }
        }

        for shard_id in to_unsubscribe {
            if shard_id == record.current_shard {
                continue;
            }

            let message = NetworkMessage::Unsubscribe {
                client_id: record.client_id,
                topic: Topic::ShardInstance { id: shard_id },
            };

            if let Ok(packet) = encode_message(&message) {
                if let Err(e) = broker.handle.send(packet) {
                    tracing::error!(
                        "AOI: failed to unsubscribe client {} from shard {}: {e:#}",
                        record.client_id.0,
                        shard_id.0
                    );
                    continue;
                }

                record.subscribed_shards.remove(&shard_id);

                tracing::debug!(
                    "AOI: unsubscribed client {} from shard {}",
                    record.client_id.0,
                    shard_id.0
                );
            }
        }
    }
}
