use bevy::prelude::*;
use crate::messages::CrossingAlertMsg;

/// Consume CrossingAlertMsg messages and log them.
///
/// Intentional stub — Part 3 (Flexible Authority) will extend this system
/// to emit HandoffRequest messages toward the destination shard.
pub fn handle_crossing_alerts(mut ev_reader: MessageReader<CrossingAlertMsg>) {
    for alert in ev_reader.read() {
        tracing::info!(
            "CrossingAlert: client_id={} is near boundary between shards {:?}",
            alert.client_id,
            alert.iter_shards(),
        );
        // Part 3: trigger HandoffRequest to destination shard here
    }
}

