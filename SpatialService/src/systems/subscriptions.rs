use bevy::prelude::*;
use shared::protocol::{ShardId};
use crate::messages::{CrossingAlertMsg, HandoffRequestMsg, PositionUpdateMsg};
use crate::net::orchestrator_client::{maybe_request_stop_shard_if_drained, split_overloaded_shard_if_needed, SplitShardResult};
use crate::resources::config::SpatialConfig;
use crate::resources::crossing_cooldowns::CrossingCooldowns;
use crate::resources::entity_map::EntityMap;
use crate::resources::handoff_queue::PendingHandoffs;
use crate::resources::net_handles::{OrchestratorClient};
use crate::resources::quad_tree::QuadTree;

/// Consume PositionUpdateMsg, update entity positions in EntityMap,
/// send Subscribe/Unsubscribe when a player entity crosses a shard boundary,
/// and emit CrossingAlertMsg for handoff detection.
pub fn handle_subscriptions(
    mut ev_positions: MessageReader<PositionUpdateMsg>,
    mut ev_crossings: MessageWriter<CrossingAlertMsg>,
    mut ev_handoffs: MessageWriter<HandoffRequestMsg>,
    mut quad_tree: ResMut<QuadTree>,
    mut entity_map: ResMut<EntityMap>,
    mut cooldowns: ResMut<CrossingCooldowns>,
    config: Res<SpatialConfig>,
    mut orchestrator: ResMut<OrchestratorClient>,
    mut pending_handoffs: ResMut<PendingHandoffs>,
) {
    cooldowns.tick();
    let mut nearby_buf: Vec<ShardId> = Vec::with_capacity(4);

    for update in ev_positions.read() {
        if !entity_map.is_stable(update.entity_id) {
            continue;
        }

        let x = update.x as f32;
        let y = update.y as f32;

        let Some(position_shard) = quad_tree.shard_for(x, y) else {
            continue;
        };

        let Some(record) = entity_map.entities.get_mut(&update.entity_id) else {
            continue;
        };

        let current_shard = record.current_shard;
        record.position = Vec2::new(x, y);

        if position_shard != current_shard {
            queue_or_emit_handoff(
                &mut ev_handoffs,
                &mut pending_handoffs,
                HandoffRequestMsg {
                    entity_id: update.entity_id,
                    from_shard: current_shard,
                    to_shard: position_shard,
                },
            );

            continue;
        }

        let current_count = entity_map.shard_count(current_shard);

        if let Some(split) = split_overloaded_shard_if_needed(
            &mut quad_tree,
            &entity_map,
            &mut orchestrator,
            current_shard,
            current_count,
        ) {
            orchestrator.mark_split_parent_candidate(split.old_shard);

            request_handoffs_after_split(
                &mut ev_handoffs,
                &mut pending_handoffs,
                &split,
            );

            maybe_request_stop_shard_if_drained(
                &mut orchestrator,
                &entity_map,
                &pending_handoffs,
                split.old_shard,
            );
        }

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
fn queue_or_emit_handoff(
    ev_handoffs: &mut MessageWriter<HandoffRequestMsg>,
    pending_handoffs: &mut PendingHandoffs,
    handoff: HandoffRequestMsg,
) {
    let to_shard = handoff.to_shard;
    let entity_id = handoff.entity_id;

    if let Some(ready_handoff) = pending_handoffs.queue_or_ready(handoff) {
        ev_handoffs.write(ready_handoff);

        tracing::info!(
            "handoff ready immediately for entity {} to connected shard {}",
            entity_id.0,
            to_shard.0,
        );
    } else {
        tracing::info!(
            "queued handoff for entity {} until shard {} connects",
            entity_id.0,
            to_shard.0,
        );
    }
}

fn request_handoffs_after_split(
    ev_handoffs: &mut MessageWriter<HandoffRequestMsg>,
    pending_handoffs: &mut PendingHandoffs,
    split: &SplitShardResult,
) {
    for moved in &split.moved_entities {
        if moved.old_shard == moved.new_shard {
            continue;
        }

        queue_or_emit_handoff(
            ev_handoffs,
            pending_handoffs,
            HandoffRequestMsg {
                entity_id: moved.entity_id,
                from_shard: moved.old_shard,
                to_shard: moved.new_shard,
            },
        );
    }
}
