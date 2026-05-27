use bevy::prelude::*;
use shared::protocol::broker::{encode_subscribe, encode_unsubscribe, topic_for_shard};
use crate::messages::{CrossingAlertMsg, PositionUpdateMsg};
use crate::resources::client_map::ClientMap;
use crate::resources::config::SpatialConfig;
use crate::resources::crossing_cooldowns::CrossingCooldowns;
use crate::resources::net_handles::BrokerClient;
use crate::resources::quad_tree::QuadTree;

/// Consume PositionUpdateMsg messages, resolve the shard via the QuadTree,
/// and send Subscribe / Unsubscribe to the broker when the shard changes.
/// Emits CrossingAlertMsg (deduplicated via cooldown) when near a boundary.
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
    let mut nearby_buf: Vec<u32> = Vec::with_capacity(4);

    for update in ev_positions.read() {
        // f64 → f32 for QuadTree (tree is built from f32 world bounds).
        let x = update.x as f32;
        let y = update.y as f32;

        let new_shard = quad_tree.shard_for(x, y);
        let old_shard = client_map.get(update.client_id);

        // Unsubscribe from the previous shard, subscribe to the new one.
        if new_shard != old_shard {
            if let Some(old) = old_shard {
                broker.send(encode_unsubscribe(update.client_id, topic_for_shard(old)));
                tracing::debug!("client {} unsubscribed from shard:{old}", update.client_id);
            }

            if let Some(new) = new_shard {
                broker.send(encode_subscribe(update.client_id, topic_for_shard(new)));
                // TODO: pass real GameConnection when shard-to-connection tracking is wired up.
                // For now we store without connection key — cleanup will happen via shard disconnect.
                client_map.shard_by_client.insert(update.client_id, new);
                tracing::debug!("client {} subscribed to shard:{new}", update.client_id);
            }
        }

        // Detect proximity to a shard boundary using the reusable buffer.
        quad_tree.shards_near_into(x, y, config.crossing_margin, &mut nearby_buf);
        if nearby_buf.len() > 1 {
            // Emit one alert per unique (client, shard_pair) combo, suppressed for cooldown_ticks.
            for i in 0..nearby_buf.len() {
                for j in (i + 1)..nearby_buf.len() {
                    if cooldowns.should_emit(update.client_id, nearby_buf[i], nearby_buf[j]) {
                        ev_crossings.write(CrossingAlertMsg::from_slice(
                            update.client_id,
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


