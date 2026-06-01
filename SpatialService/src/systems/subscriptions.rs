use bevy::prelude::*;
use shared::protocol::broker::{encode_message, BrokerMessage, ShardId};
use crate::messages::{CrossingAlertMsg, PositionUpdateMsg};
use crate::resources::client_map::ClientMap;
use crate::resources::config::SpatialConfig;
use crate::resources::crossing_cooldowns::CrossingCooldowns;
use crate::resources::net_handles::BrokerClient;
use crate::resources::quad_tree::QuadTree;

/// Consume PositionUpdateMsg messages, resolve the shard via the QuadTree,
/// and send Subscribe / Unsubscribe to the broker when the shard changes.
/// Emits CrossingAlertMsg (deduplicated via cooldown) when near a boundary.
/// Clients in `PendingHandoff` state are skipped to avoid conflicting sub changes.
pub fn handle_subscriptions(
    mut ev_positions: MessageReader<PositionUpdateMsg>,
    mut ev_crossings: MessageWriter<CrossingAlertMsg>,
    quad_tree: Res<QuadTree>,
    mut client_map: ResMut<ClientMap>,
    mut cooldowns: ResMut<CrossingCooldowns>,
    broker: Res<BrokerClient>,
    config: Res<SpatialConfig>,
) {
    // Tick down cooldowns once per frame before processing positions.
    cooldowns.tick();

    // Reusable buffer — allocated once per frame, not once per client.
    let mut nearby_buf: Vec<ShardId> = Vec::with_capacity(4);

    for update in ev_positions.read() {
        // Skip clients mid-handoff — their subscription will be updated by the handoff system.
        if !client_map.is_stable(update.client_id.into()) {
            continue;
        }

        // f64 → f32 for QuadTree (tree is built from f32 world bounds).
        let x = update.x as f32;
        let y = update.y as f32;

        let new_shard = quad_tree.shard_for(x, y);
        let old_shard = client_map.get(update.client_id.into());

        // Unsubscribe from the previous shard, subscribe to the new one.
        if new_shard != old_shard {
            if let Some(old) = old_shard {
                let packet = match encode_message(&BrokerMessage::Unsubscribe {
                    client_id: update.client_id,
                    shard_id: old,
                }) {
                    Ok(packet) => packet,
                    Err(error) => {
                        tracing::error!("failed to encode Unsubscribe for client {}: {error}", update.client_id.0);
                        continue;
                    }
                };
                broker.send(packet);
                tracing::debug!("client {} unsubscribed from shard:{}", update.client_id.0, old.0);
            }

            if let Some(new) = new_shard {
                let packet = match encode_message(&BrokerMessage::Subscribe {
                    client_id: update.client_id,
                    shard_id: new,
                }) {
                    Ok(packet) => packet,
                    Err(error) => {
                        tracing::error!("failed to encode Subscribe for client {}: {error}", update.client_id.0);
                        continue;
                    }
                };
                broker.send(packet);

                // Use the proper insert() method to maintain the connection_clients index
                // so that remove_by_connection() correctly cleans up on shard disconnect.
                if let Some(conn) = update.shard_connection {
                    client_map.insert(update.client_id.into(), new, conn);
                } else {
                    // Broker-relayed path: no direct connection available, update shard only.
                    client_map.shard_by_client.insert(update.client_id.into(), new);
                }
                tracing::debug!("client {} subscribed to shard:{}", update.client_id.0, new.0);
            }
        }

        // Detect proximity to a shard boundary using the reusable buffer.
        quad_tree.shards_near_into(x, y, config.crossing_margin, &mut nearby_buf);
        if nearby_buf.len() > 1 {
            // Emit one alert per unique (client, shard_pair) combo, suppressed for cooldown_ticks.
            for i in 0..nearby_buf.len() {
                for j in (i + 1)..nearby_buf.len() {
                    if cooldowns.should_emit(update.client_id.into(), nearby_buf[i].0, nearby_buf[j].0) {
                        ev_crossings.write(CrossingAlertMsg::from_slice(
                            update.client_id.into(),
                            &nearby_buf,
                        ));
                        // One alert per client per tick is sufficient.
                        break;
                    }
                }
            }
        }
    }
}
