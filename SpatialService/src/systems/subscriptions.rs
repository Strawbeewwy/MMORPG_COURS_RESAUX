use bevy::prelude::*;
use shared::protocol::{encode_message, NetworkMessage, ShardId, Topic};
use crate::messages::{CrossingAlertMsg, PositionUpdateMsg};
use crate::resources::config::SpatialConfig;
use crate::resources::crossing_cooldowns::CrossingCooldowns;
use crate::resources::entity_map::EntityMap;
use crate::resources::net_handles::BrokerClient;
use crate::resources::quad_tree::QuadTree;

/// Consume PositionUpdateMsg, update entity positions in EntityMap,
/// send Subscribe/Unsubscribe when a player entity crosses a shard boundary,
/// and emit CrossingAlertMsg for handoff detection.
pub fn handle_subscriptions(
    mut ev_positions: MessageReader<PositionUpdateMsg>,
    mut ev_crossings: MessageWriter<CrossingAlertMsg>,
    quad_tree: Res<QuadTree>,
    mut entity_map: ResMut<EntityMap>,
    mut cooldowns: ResMut<CrossingCooldowns>,
    broker: Res<BrokerClient>,
    config: Res<SpatialConfig>,
) {
    cooldowns.tick();
    let mut nearby_buf: Vec<ShardId> = Vec::with_capacity(4);

    for update in ev_positions.read() {
        // Skip entities mid-handoff — subscription managed by HandoffCompleted handler.
        if !entity_map.is_stable(update.entity_id) {
            continue;
        }

        let x = update.x as f32;
        let y = update.y as f32;

        let Some(new_shard) = quad_tree.shard_for(x, y) else {
            continue;
        };

        let Some(record) = entity_map.entities.get_mut(&update.entity_id) else {
            continue;
        };

        let old_shard = record.current_shard;

        if new_shard != old_shard {
            if record.is_player() {
                let client_id = record.client_id;

                if let Ok(packet) = encode_message(&NetworkMessage::Unsubscribe {
                    client_id,
                    topic: Topic::ShardInstance { id: old_shard },
                }) {
                    if let Err(e) = broker.handle.send(packet) {
                        tracing::error!("failed to send Unsubscribe for entity {}: {e}", update.entity_id.0);
                    }
                }

                if let Ok(packet) = encode_message(&NetworkMessage::Subscribe {
                    client_id,
                    topic: Topic::ShardInstance { id: new_shard },
                }) {
                    if let Err(e) = broker.handle.send(packet) {
                        tracing::error!("failed to send Subscribe for entity {}: {e}", update.entity_id.0);
                    }
                }
            }

            record.current_shard = new_shard;
        }

        record.position = Vec2::new(x, y);

        // Emit crossing alert when near a shard boundary.
        quad_tree.shards_near_into(x, y, config.crossing_margin, &mut nearby_buf);
        if nearby_buf.len() > 1 {
            for i in 0..nearby_buf.len() {
                for j in (i + 1)..nearby_buf.len() {
                    if cooldowns.should_emit(update.entity_id.0, nearby_buf[i].0, nearby_buf[j].0) {
                        ev_crossings.write(CrossingAlertMsg::from_slice(
                            update.entity_id,
                            &nearby_buf,
                        ));
                        break;
                    }
                }
            }
        }
    }
}
