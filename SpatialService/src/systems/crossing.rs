use bevy::prelude::*;
use shared::protocol::ShardId;
use crate::messages::{CrossingAlertMsg, HandoffRequestMsg};
use crate::resources::client_map::ClientMap;

/// Consume CrossingAlertMsg events and emit HandoffRequestMsg for each client
/// that is stable and near a shard boundary.
///
/// Only one HandoffRequest is emitted per crossing event — for the first
/// neighboring shard that differs from the client's current shard.
/// Clients already in `PendingHandoff` state are skipped.
pub fn handle_crossing_alerts(
    mut ev_reader: MessageReader<CrossingAlertMsg>,
    mut ev_handoff: MessageWriter<HandoffRequestMsg>,
    client_map: Res<ClientMap>,
) {
    for alert in ev_reader.read() {
        let client_id = alert.client_id;

        // Skip clients already mid-handoff to avoid duplicate HandoffRequest messages.
        if !client_map.is_stable(client_id.into()) {
            tracing::debug!(
                "CrossingAlert for client {} skipped — handoff already in progress",
                client_id.0
            );
            continue;
        }

        let Some(current_shard) = client_map.get(client_id.into()) else {
            tracing::debug!(
                "CrossingAlert for client {} skipped — not yet subscribed to any shard",
                client_id.0
            );
            continue;
        };

        // Pick the first neighbouring shard that differs from the current one.
        let Some(&to_shard) = alert
            .iter_shards()
            .iter()
            .find(|s| s.0 != current_shard.0)
        else {
            continue;
        };

        tracing::info!(
            "CrossingAlert: client {} crossing from shard {} → shard {}",
            client_id.0, current_shard.0, to_shard.0
        );

        ev_handoff.write(HandoffRequestMsg {
            client_id,
            from_shard: ShardId(current_shard.0),
            to_shard,
        });
    }
}
