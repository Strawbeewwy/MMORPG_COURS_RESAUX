use bevy::prelude::*;
use bevy::platform::collections::HashSet;
use shared::game_sockets::{
    GameConnection, GameNetworkEvent, GameStreamReliability
};
use shared::{NetVec2, ShardId};
use shared::protocol::{Topic, decode_message, encode_message, NetworkMessage};
use crate::messages::{HandoffRequestMsg, PositionUpdateMsg};
use crate::net::orchestrator_client::{maybe_request_stop_shard_if_drained, split_overloaded_shard_if_needed};
use crate::resources::entity_map::{EntityMap, EntityTransferState, SpatialEntityRecord};
use crate::resources::handoff_queue::PendingHandoffs;
use crate::resources::net_handles::{BrokerClient, BrokerConnectionState, OrchestratorClient};
use crate::resources::quad_tree::QuadTree;

/// Poll the broker peer to advance handshake state and maintain the connection.
pub fn poll_broker_connection(
    mut broker: ResMut<BrokerClient>,
    mut entity_map: ResMut<EntityMap>,
    mut ev_writer: MessageWriter<PositionUpdateMsg>,
    mut ev_handoffs: MessageWriter<HandoffRequestMsg>,
    mut quad_tree: ResMut<QuadTree>,
    mut orchestrator: ResMut<OrchestratorClient>,
    mut pending_handoffs: ResMut<PendingHandoffs>,
) {
    loop {
        match broker.handle.peer.poll() {
            Ok(Some(event)) => {
                handle_broker_event(
                    &mut broker,
                    event,
                    &mut ev_writer,
                    &mut ev_handoffs,
                    &mut entity_map,
                    &mut quad_tree,
                    &mut orchestrator,
                    &mut pending_handoffs,
                )
            },
            Ok(None) => break,
            Err(e) => {
                tracing::error!("utils client poll error: {e}");
                break;
            }
        }
    }
}

fn handle_broker_event(
    broker: &mut BrokerClient,
    event: GameNetworkEvent,
    ev_writer: &mut MessageWriter<PositionUpdateMsg>,
    ev_handoffs: &mut MessageWriter<HandoffRequestMsg>,
    entity_map: &mut EntityMap,
    quad_tree: &mut QuadTree,
    orchestrator: &mut OrchestratorClient,
    pending_handoffs: &mut PendingHandoffs,
) {
    match event {
        GameNetworkEvent::Connected(conn) => {
            tracing::info!("connected to utils: {}", conn.connection_id);
            broker.handle.connection = Some(conn);
            broker.handle.state = BrokerConnectionState::Connected;
            if let Err(e) = broker.handle.peer.create_stream(conn, GameStreamReliability::Reliable) {
                tracing::error!("failed to create stream towards utils: {e}");
            }
        }
        GameNetworkEvent::Disconnected(_conn) => {
            tracing::warn!("utils connection lost — will reconnect next tick");
            broker.handle.connection = None;
            broker.handle.stream = None;
            broker.handle.state = BrokerConnectionState::Disconnected;
            pending_handoffs.mark_all_disconnected();
        }
        GameNetworkEvent::StreamCreated(_conn, stream) => {
            tracing::info!("utils stream ready");
            broker.handle.stream = Some(stream);
            broker.handle.state = BrokerConnectionState::Ready;
            broker.handle.reset_backoff();
        }
        GameNetworkEvent::StreamClosed(_conn, _stream) => {
            broker.handle.stream = None;
            broker.handle.state = BrokerConnectionState::Disconnected;
            pending_handoffs.mark_all_disconnected();
        }
        GameNetworkEvent::Error { connection, inner } => {
            tracing::warn!("utils error on {}: {inner}", connection.connection_id);
            broker.handle.state = BrokerConnectionState::Disconnected;
            pending_handoffs.mark_all_disconnected();
        }
        GameNetworkEvent::Message { connection, data, .. } => {
            handle_broker_message(
                connection,
                &data,
                ev_writer,
                ev_handoffs,
                entity_map,
                quad_tree,
                broker,
                orchestrator,
                pending_handoffs,
            );
        }
    }
}

/// Handle a message received via the utils relay path.
/// PositionUpdates here have no direct shard connection — shard_connection is None.
pub fn handle_broker_message(
    connection: GameConnection,
    data: &[u8],
    ev_writer: &mut MessageWriter<PositionUpdateMsg>,
    ev_handoffs: &mut MessageWriter<HandoffRequestMsg>,
    entity_map: &mut EntityMap,
    quad_tree: &mut QuadTree,
    broker: &mut BrokerClient,
    orchestrator: &mut OrchestratorClient,
    pending_handoffs: &mut PendingHandoffs,
) {
    let message = match decode_message(data) {
        Ok(msg) => msg,
        Err(e) => {
            tracing::warn!("invalid broker message from connection {}: {e}", connection.connection_id);
            return;
        }
    };

    match message {
        NetworkMessage::RegisterShard { shard_id } => {
            let ready_handoffs = pending_handoffs.mark_connected(shard_id);
            let ready_count = ready_handoffs.len();

            for handoff in ready_handoffs {
                if !entity_map.is_stable(handoff.entity_id) {
                    continue;
                }

                ev_handoffs.write(handoff);
            }

            tracing::info!(
                "shard {} connected; flushed {} pending handoff(s)",
                shard_id.0,
                ready_count,
            );
        }

        NetworkMessage::UnregisterShard { shard_id } => {
            pending_handoffs.mark_disconnected(shard_id);

            tracing::warn!(
                "shard {} disconnected; pending handoffs for this shard={}",
                shard_id.0,
                pending_handoffs.pending_count_for(shard_id),
            );
        }

        NetworkMessage::PositionUpdate { entity_id, position } => {
            ev_writer.write(PositionUpdateMsg {
                entity_id,
                x: f64::from(position.x),
                y: f64::from(position.y),
            });
        }
        NetworkMessage::RegisterEntity {entity_id, client_id, position} => {

            let f32_position= Vec2::from(NetVec2::to_f32(&position));

            let Some(shard) = quad_tree.shard_for(f32_position.x,f32_position.y) else {
                tracing::warn!("no shard for specified position : x{} y{}",
                    f32_position.x,
                    f32_position.y
                );
                return;
            };

            let mut subscribed_shards = HashSet::new();
            subscribed_shards.insert(shard);

            let record = SpatialEntityRecord {
                entity_id,
                client_id,
                position : f32_position,
                current_shard: shard,
                subscribed_shards,
            };
            entity_map.insert(entity_id, record);

            let shard_count = entity_map.shard_count(shard);

            if let Some(split) = split_overloaded_shard_if_needed(
                quad_tree,
                entity_map,
                orchestrator,
                shard,
                shard_count,
            ) {
                orchestrator.mark_split_parent_candidate(split.old_shard);

                queue_or_emit_split_handoffs(
                    ev_handoffs,
                    pending_handoffs,
                    entity_map,
                    split.clone(),
                );

                maybe_request_stop_shard_if_drained(
                    orchestrator,
                    entity_map,
                    pending_handoffs,
                    split.old_shard,
                );
            }
        }
        NetworkMessage::UnregisterEntity {entity_id} => {
            pending_handoffs.remove_entity(entity_id);
            entity_map.remove(entity_id);
        }
        NetworkMessage::HandoffCompleted { entity_id } => {
            pending_handoffs.remove_entity(entity_id);

            // Broker CCs spatial on HandoffCompleted so it can update subscriptions.
            if let EntityTransferState::PendingHandoff { destination_shard } = entity_map.get_state(entity_id) {
                let dest_shard_id = ShardId(destination_shard);

                let Some(record) = entity_map.entities.get(&entity_id) else {
                    entity_map.clear_state(entity_id);
                    return;
                };

                let client_id = record.client_id;
                let old_shard = record.current_shard;
                let is_player = record.is_player();

                if is_player {
                    // Unsubscribe from source shard.
                    if let Ok(packet) = encode_message(&NetworkMessage::Unsubscribe {
                        client_id,
                        topic: Topic::ShardInstance { id: old_shard },
                    }) {
                        if let Err(e) = broker.handle.send(packet) {
                            tracing::error!("HandoffCompleted: failed unsubscribe: {e:#}");
                        }
                    }

                    // Subscribe to destination shard.
                    if let Ok(packet) = encode_message(&NetworkMessage::Subscribe {
                        client_id,
                        topic: Topic::ShardInstance { id: dest_shard_id },
                    }) {
                        if let Err(e) = broker.handle.send(packet) {
                            tracing::error!("HandoffCompleted: failed subscribe: {e:#}");
                        }
                    }
                }

                if let Some((_old_shard, new_shard, new_count)) =
                    entity_map.move_entity_to_shard(entity_id, dest_shard_id)
                {
                    if let Some(split) = split_overloaded_shard_if_needed(
                        quad_tree,
                        entity_map,
                        orchestrator,
                        new_shard,
                        new_count,
                    ) {
                        orchestrator.mark_split_parent_candidate(split.old_shard);

                        queue_or_emit_split_handoffs(
                            ev_handoffs,
                            pending_handoffs,
                            entity_map,
                            split,
                        );
                    }

                    maybe_request_stop_shard_if_drained(
                        orchestrator,
                        entity_map,
                        pending_handoffs,
                        old_shard,
                    );
                }

                entity_map.clear_state(entity_id);

                tracing::info!(
                    "HandoffCompleted: entity {} moved to shard {}",
                    entity_id.0, destination_shard
                );
            }
        }
        _ => {}
    }
}


fn queue_or_emit_split_handoffs(
    ev_handoffs: &mut MessageWriter<HandoffRequestMsg>,
    pending_handoffs: &mut PendingHandoffs,
    entity_map: &EntityMap,
    split: crate::net::orchestrator_client::SplitShardResult,
) {
    for moved in split.moved_entities {
        if moved.old_shard == moved.new_shard {
            continue;
        }

        if !entity_map.is_stable(moved.entity_id) {
            continue;
        }

        let handoff = HandoffRequestMsg {
            entity_id: moved.entity_id,
            from_shard: moved.old_shard,
            to_shard: moved.new_shard,
        };

        if let Some(ready_handoff) = pending_handoffs.queue_or_ready(handoff) {
            ev_handoffs.write(ready_handoff);

            tracing::info!(
                "split: emitted handoff for entity {} from shard {} to connected shard {}",
                moved.entity_id.0,
                moved.old_shard.0,
                moved.new_shard.0,
            );
        } else {
            tracing::info!(
                "split: queued handoff for entity {} from shard {} until shard {} connects",
                moved.entity_id.0,
                moved.old_shard.0,
                moved.new_shard.0,
            );
        }
    }
}
